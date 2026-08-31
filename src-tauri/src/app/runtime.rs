//! Managed runtime for one-shot native provider collection.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::collection::{
    CollectionClock, CollectionCoordinator, CollectionError, CollectionReport, ProviderSource,
};
use crate::database::connection::{IndexStore, StorageError};
use crate::providers::registry::provider_registry;
use crate::sources::file_watcher::WatchRoot;
use crate::sources::provider_roots::{configured_root_path, resolve_configured_root};
use crate::sources::session_files::{discover_configured_source, DiscoveryLimits};
use crate::sources::source_config::{SourceConfig, SourceConfigSet};
use crate::types::provider::Provider;
use crate::types::usage_summary::UsageSummary;
use crate::types::widget_settings::WidgetSettingsSnapshot;

pub const DEFAULT_DISCOVERY_LIMITS: DiscoveryLimits =
    DiscoveryLimits::without_file_count(50 * 1024 * 1024);

struct Runtime {
    coordinator: CollectionCoordinator<IndexStore>,
    profile_root: PathBuf,
    discovery_limits: DiscoveryLimits,
    source_configs: SourceConfigSet,
    invalid_settings: Vec<Provider>,
    widget_settings: WidgetSettingsSnapshot,
}

#[derive(Clone)]
pub struct AppState {
    runtime: Arc<Mutex<Option<Runtime>>>,
    fallback_summary: UsageSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeInitError {
    ProfileUnavailable,
    DataDirectory,
    DatabaseOpen,
    SettingsRead,
}

impl fmt::Display for RuntimeInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::ProfileUnavailable => "profile_unavailable",
            Self::DataDirectory => "data_directory",
            Self::DatabaseOpen => "database_open",
            Self::SettingsRead => "settings_read",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for RuntimeInitError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Unavailable,
    StatePoisoned,
    Settings(StorageError),
    Collection(CollectionError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("unavailable"),
            Self::StatePoisoned => formatter.write_str("state_poisoned"),
            Self::Settings(error) => write!(formatter, "settings:{error}"),
            Self::Collection(error) => write!(formatter, "collection:{error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<CollectionError> for RuntimeError {
    fn from(error: CollectionError) -> Self {
        Self::Collection(error)
    }
}

impl Runtime {
    fn collect_once(
        &mut self,
        clock: &dyn CollectionClock,
    ) -> Result<CollectionReport, CollectionError> {
        let registry = provider_registry();
        let sources = registry
            .registrations()
            .map(|registration| {
                let provider = registration.provider();
                let config = self.source_configs.get(provider);
                let discovery =
                    discover_configured_source(&self.profile_root, config, self.discovery_limits);
                ProviderSource::with_configured_root(
                    config.enabled(),
                    config.configured_root_label(),
                    self.invalid_settings.contains(&provider),
                    discovery,
                    registration.adapter(),
                )
            })
            .collect::<Vec<_>>();
        let report = self.coordinator.collect(&sources, clock)?;
        self.invalid_settings.clear();
        Ok(report)
    }

    fn watch_roots(&self) -> Vec<WatchRoot> {
        provider_registry()
            .providers()
            .filter_map(|provider| {
                let config = self.source_configs.get(provider);
                if !config.enabled() {
                    return None;
                }
                resolve_configured_root(&self.profile_root, config)
                    .ok()
                    .map(|root| WatchRoot::new(provider, root.filesystem_path().to_path_buf()))
            })
            .collect()
    }

    fn source_config(&self, provider: Provider) -> SourceConfig {
        self.source_configs.get(provider).clone()
    }

    fn update_source_config(&mut self, config: SourceConfig) -> Result<(), RuntimeError> {
        self.coordinator
            .save_source_config(&config)
            .map_err(RuntimeError::Settings)?;
        self.source_configs.replace(config.clone());
        self.invalid_settings
            .retain(|provider| *provider != config.provider());
        Ok(())
    }

    fn update_widget_settings(
        &mut self,
        settings: WidgetSettingsSnapshot,
    ) -> Result<(), RuntimeError> {
        self.coordinator
            .save_widget_settings(&settings)
            .map_err(RuntimeError::Settings)?;
        self.widget_settings = settings;
        Ok(())
    }
}

impl AppState {
    pub fn from_paths(
        profile_root: PathBuf,
        database_path: &Path,
        discovery_limits: DiscoveryLimits,
    ) -> Result<Self, RuntimeInitError> {
        if !profile_root.is_dir() {
            return Err(RuntimeInitError::ProfileUnavailable);
        }

        let database_parent = database_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(database_parent).map_err(|_| RuntimeInitError::DataDirectory)?;
        let store = IndexStore::open(database_path).map_err(|_| RuntimeInitError::DatabaseOpen)?;
        let loaded = store
            .load_source_configs()
            .map_err(|_| RuntimeInitError::SettingsRead)?;
        let widget_settings = store
            .load_widget_settings()
            .map_err(|_| RuntimeInitError::SettingsRead)?;

        Ok(Self {
            runtime: Arc::new(Mutex::new(Some(Runtime {
                coordinator: CollectionCoordinator::new(store),
                profile_root,
                discovery_limits,
                source_configs: loaded.configs,
                invalid_settings: loaded.invalid_providers,
                widget_settings,
            }))),
            fallback_summary: UsageSummary::unavailable(),
        })
    }

    pub fn unavailable() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(None)),
            fallback_summary: UsageSummary::unavailable(),
        }
    }

    pub fn collect_once(
        &self,
        clock: &dyn CollectionClock,
    ) -> Result<CollectionReport, RuntimeError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?;
        let runtime = runtime.as_mut().ok_or(RuntimeError::Unavailable)?;
        runtime
            .collect_once(clock)
            .map_err(RuntimeError::Collection)
    }

    pub fn source_config(&self, provider: Provider) -> Result<SourceConfig, RuntimeError> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?;
        runtime
            .as_ref()
            .map(|runtime| runtime.source_config(provider))
            .ok_or(RuntimeError::Unavailable)
    }

    pub fn source_root_path(&self, provider: Provider) -> Result<PathBuf, RuntimeError> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?;
        let runtime = runtime.as_ref().ok_or(RuntimeError::Unavailable)?;
        configured_root_path(&runtime.profile_root, runtime.source_configs.get(provider))
            .map_err(|_| RuntimeError::Unavailable)
    }

    pub fn update_source_config(&self, config: SourceConfig) -> Result<(), RuntimeError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?;
        runtime
            .as_mut()
            .ok_or(RuntimeError::Unavailable)?
            .update_source_config(config)
    }

    pub fn widget_settings(&self) -> Result<WidgetSettingsSnapshot, RuntimeError> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?;
        runtime
            .as_ref()
            .map(|runtime| runtime.widget_settings.clone())
            .ok_or(RuntimeError::Unavailable)
    }

    pub fn update_widget_settings(
        &self,
        settings: WidgetSettingsSnapshot,
    ) -> Result<(), RuntimeError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?;
        runtime
            .as_mut()
            .ok_or(RuntimeError::Unavailable)?
            .update_widget_settings(settings)
    }

    pub(crate) fn watch_roots(&self) -> Vec<WatchRoot> {
        let Ok(runtime) = self.runtime.lock() else {
            return Vec::new();
        };
        runtime
            .as_ref()
            .map(Runtime::watch_roots)
            .unwrap_or_default()
    }

    pub fn summary(&self) -> UsageSummary {
        let Ok(runtime) = self.runtime.lock() else {
            return self.fallback_summary.clone();
        };
        runtime
            .as_ref()
            .map(|runtime| runtime.coordinator.last_summary().clone())
            .unwrap_or_else(|| self.fallback_summary.clone())
    }
}

pub fn initialize_from_app(app: &tauri::AppHandle) -> AppState {
    let profile_root = std::env::var_os("USERPROFILE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let database_path = app
        .path()
        .app_local_data_dir()
        .ok()
        .map(|directory| directory.join("index.sqlite"));

    let (Some(profile_root), Some(database_path)) = (profile_root, database_path) else {
        return AppState::unavailable();
    };

    AppState::from_paths(profile_root, &database_path, DEFAULT_DISCOVERY_LIMITS)
        .unwrap_or_else(|_| AppState::unavailable())
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_DISCOVERY_LIMITS;

    #[test]
    fn default_discovery_does_not_cap_file_count() {
        assert_eq!(DEFAULT_DISCOVERY_LIMITS.max_files, usize::MAX);
    }
}
