//! Managed runtime for native provider collection and shared application state.

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
use crate::sources::provider_roots::{configured_root_path, watch_root_path};
use crate::sources::session_files::{discover_configured_source, DiscoveryLimits};
use crate::sources::source_config::{SourceConfig, SourceConfigSet};
use crate::types::provider::Provider;
use crate::types::usage_summary::UsageSummary;
use crate::types::widget_settings::WidgetSettingsSnapshot;

pub const DEFAULT_DISCOVERY_LIMITS: DiscoveryLimits = DiscoveryLimits::without_file_count(u64::MAX);

struct Runtime {
    coordinator: CollectionCoordinator<IndexStore>,
    profile_root: PathBuf,
    discovery_limits: DiscoveryLimits,
    invalid_settings: Vec<Provider>,
    widget_settings: WidgetSettingsSnapshot,
}

#[derive(Clone)]
pub struct AppState {
    runtime: Arc<Mutex<Option<Runtime>>>,
    source_configs: Arc<Mutex<SourceConfigSet>>,
    base_summary: Arc<Mutex<UsageSummary>>,
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
        source_configs: &SourceConfigSet,
        clock: &dyn CollectionClock,
    ) -> Result<CollectionReport, CollectionError> {
        let registry = provider_registry();
        let sources = registry
            .registrations()
            .map(|registration| {
                let provider = registration.provider();
                let config = source_configs.get(provider);
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

    fn watch_roots(&self, source_configs: &SourceConfigSet) -> Vec<WatchRoot> {
        provider_registry()
            .providers()
            .filter_map(|provider| {
                let config = source_configs.get(provider);
                if !config.enabled() {
                    return None;
                }
                watch_root_path(&self.profile_root, config)
                    .ok()
                    .map(|path| WatchRoot::new(provider, path))
            })
            .collect()
    }

    fn update_source_config(&mut self, config: SourceConfig) -> Result<(), RuntimeError> {
        self.coordinator
            .save_source_config(&config)
            .map_err(RuntimeError::Settings)?;
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
                invalid_settings: loaded.invalid_providers,
                widget_settings,
            }))),
            source_configs: Arc::new(Mutex::new(loaded.configs)),
            base_summary: Arc::new(Mutex::new(UsageSummary::loading())),
            fallback_summary: UsageSummary::unavailable(),
        })
    }

    pub fn unavailable() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(None)),
            source_configs: Arc::new(Mutex::new(SourceConfigSet::defaults())),
            base_summary: Arc::new(Mutex::new(UsageSummary::unavailable())),
            fallback_summary: UsageSummary::unavailable(),
        }
    }

    pub fn collect_once(
        &self,
        clock: &dyn CollectionClock,
    ) -> Result<CollectionReport, RuntimeError> {
        let source_configs = self
            .source_configs
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .clone();
        let (result, base_summary) = {
            let mut runtime = self
                .runtime
                .lock()
                .map_err(|_| RuntimeError::StatePoisoned)?;
            let runtime = runtime.as_mut().ok_or(RuntimeError::Unavailable)?;
            let result = runtime
                .collect_once(&source_configs, clock)
                .map_err(RuntimeError::Collection);
            let base_summary = match &result {
                Ok(report) => report.summary.clone(),
                Err(_) => runtime.coordinator.last_summary().clone(),
            };
            (result, base_summary)
        };
        self.base_summary
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .clone_from(&base_summary);
        result.map(|mut report| {
            report.summary = self.summary();
            report
        })
    }

    pub fn source_config(&self, provider: Provider) -> Result<SourceConfig, RuntimeError> {
        self.source_configs
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)
            .and_then(|configs| {
                if self.runtime_is_available() {
                    Ok(configs.get(provider).clone())
                } else {
                    Err(RuntimeError::Unavailable)
                }
            })
    }

    pub fn source_root_path(&self, provider: Provider) -> Result<PathBuf, RuntimeError> {
        let source_configs = self
            .source_configs
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .clone();
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?;
        let runtime = runtime.as_ref().ok_or(RuntimeError::Unavailable)?;
        configured_root_path(&runtime.profile_root, source_configs.get(provider))
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
            .update_source_config(config.clone())?;
        drop(runtime);
        self.source_configs
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .replace(config);
        Ok(())
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
        let Ok(source_configs) = self.source_configs.lock() else {
            return Vec::new();
        };
        let Ok(runtime) = self.runtime.lock() else {
            return Vec::new();
        };
        runtime
            .as_ref()
            .map(|runtime| runtime.watch_roots(&source_configs))
            .unwrap_or_default()
    }

    pub fn summary(&self) -> UsageSummary {
        let Ok(base_summary) = self.base_summary.lock() else {
            return self.fallback_summary.clone();
        };
        base_summary.clone()
    }

    fn runtime_is_available(&self) -> bool {
        self.runtime
            .lock()
            .ok()
            .is_some_and(|runtime| runtime.is_some())
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
        assert_eq!(DEFAULT_DISCOVERY_LIMITS.max_total_bytes, u64::MAX);
    }
}
