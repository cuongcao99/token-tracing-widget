//! Managed runtime for native provider collection and shared application state.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::collection::{
    CollectionClock, CollectionCoordinator, CollectionError, CollectionReport, ProviderSource,
};
use crate::database::store::{IndexStore, StorageError};
use crate::providers::registry::provider_registry;
use crate::sources::file_watcher::WatchRoot;
use crate::sources::provider_roots::{
    configured_root_path, configured_root_path_for, watch_root_paths,
};
use crate::sources::session_files::{discover_configured_sources, DiscoveryLimits};
use crate::sources::source_config::{SourceConfig, SourceConfigSet, SourcePlatform};
use crate::types::provider::Provider;
use crate::types::update_settings::UpdateSettingsSnapshot;
use crate::types::usage_summary::UsageSummary;
use crate::types::widget_settings::WidgetSettingsSnapshot;

pub const DEFAULT_DISCOVERY_LIMITS: DiscoveryLimits = DiscoveryLimits::without_file_count(u64::MAX);

struct Runtime {
    coordinator: CollectionCoordinator<IndexStore>,
    profile_root: PathBuf,
    discovery_limits: DiscoveryLimits,
    invalid_settings: Vec<Provider>,
}

#[derive(Clone)]
pub struct AppState {
    runtime: Arc<Mutex<Option<Runtime>>>,
    profile_root: Option<PathBuf>,
    source_configs: Arc<Mutex<SourceConfigSet>>,
    update_settings: Arc<Mutex<Option<UpdateSettingsSnapshot>>>,
    widget_settings: Arc<Mutex<Option<WidgetSettingsSnapshot>>>,
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
                let discoveries =
                    discover_configured_sources(&self.profile_root, config, self.discovery_limits);
                let configured_root = discoveries
                    .iter()
                    .map(|discovery| discovery.configured_root())
                    .collect::<Vec<_>>()
                    .join(" + ");
                ProviderSource::with_discoveries(
                    config.enabled(),
                    configured_root,
                    self.invalid_settings.contains(&provider),
                    discoveries,
                    registration.adapter(),
                )
            })
            .collect::<Vec<_>>();
        let report = self.coordinator.collect(&sources, clock)?;
        self.invalid_settings.clear();
        Ok(report)
    }

    fn update_source_config(&mut self, config: SourceConfig) -> Result<(), RuntimeError> {
        self.coordinator
            .store_mut()
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
            .store_mut()
            .save_widget_settings(&settings)
            .map_err(RuntimeError::Settings)
    }

    fn save_update_settings(
        &mut self,
        settings: &UpdateSettingsSnapshot,
    ) -> Result<(), RuntimeError> {
        self.coordinator
            .store_mut()
            .save_update_settings(settings)
            .map_err(RuntimeError::Settings)
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
        let update_settings = store
            .load_update_settings()
            .map_err(|_| RuntimeInitError::SettingsRead)?;

        Ok(Self {
            runtime: Arc::new(Mutex::new(Some(Runtime {
                coordinator: CollectionCoordinator::new(store),
                profile_root: profile_root.clone(),
                discovery_limits,
                invalid_settings: loaded.invalid_providers,
            }))),
            profile_root: Some(profile_root),
            source_configs: Arc::new(Mutex::new(loaded.configs)),
            update_settings: Arc::new(Mutex::new(Some(update_settings))),
            widget_settings: Arc::new(Mutex::new(Some(widget_settings))),
            base_summary: Arc::new(Mutex::new(UsageSummary::loading())),
            fallback_summary: UsageSummary::unavailable(),
        })
    }

    pub fn unavailable() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(None)),
            profile_root: None,
            source_configs: Arc::new(Mutex::new(SourceConfigSet::defaults())),
            update_settings: Arc::new(Mutex::new(None)),
            widget_settings: Arc::new(Mutex::new(None)),
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
        if self.profile_root.is_none() {
            return Err(RuntimeError::Unavailable);
        }
        self.source_configs
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)
            .map(|configs| configs.get(provider).clone())
    }

    pub fn source_root_path(&self, provider: Provider) -> Result<PathBuf, RuntimeError> {
        let source_configs = self
            .source_configs
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .clone();
        let profile_root = self
            .profile_root
            .as_ref()
            .ok_or(RuntimeError::Unavailable)?;
        configured_root_path(profile_root, source_configs.get(provider))
            .map_err(|_| RuntimeError::Unavailable)
    }

    pub fn source_root_path_for(
        &self,
        provider: Provider,
        platform: SourcePlatform,
    ) -> Result<PathBuf, RuntimeError> {
        let source_configs = self
            .source_configs
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .clone();
        let profile_root = self
            .profile_root
            .as_ref()
            .ok_or(RuntimeError::Unavailable)?;
        configured_root_path_for(profile_root, source_configs.get(provider), platform)
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
        self.widget_settings
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .clone()
            .ok_or(RuntimeError::Unavailable)
    }

    pub fn update_settings(&self) -> Result<UpdateSettingsSnapshot, RuntimeError> {
        self.update_settings
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .clone()
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
            .update_widget_settings(settings.clone())?;
        self.widget_settings
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .replace(settings);
        Ok(())
    }

    pub fn save_update_settings(
        &self,
        settings: UpdateSettingsSnapshot,
    ) -> Result<(), RuntimeError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?;
        runtime
            .as_mut()
            .ok_or(RuntimeError::Unavailable)?
            .save_update_settings(&settings)?;
        self.update_settings
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .replace(settings);
        Ok(())
    }

    pub(crate) fn watch_roots(&self) -> Vec<WatchRoot> {
        let Some(profile_root) = self.profile_root.as_ref() else {
            return Vec::new();
        };
        let Ok(source_configs) = self.source_configs.lock() else {
            return Vec::new();
        };
        provider_registry()
            .providers()
            .flat_map(|provider| {
                let config = source_configs.get(provider);
                watch_root_paths(profile_root, config)
                    .into_iter()
                    .map(move |path| WatchRoot::new(provider, path))
            })
            .collect()
    }

    pub fn summary(&self) -> UsageSummary {
        let Ok(base_summary) = self.base_summary.lock() else {
            return self.fallback_summary.clone();
        };
        base_summary.clone()
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
