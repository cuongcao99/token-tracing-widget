//! Managed runtime for one-shot native provider collection.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::collection::{
    CollectionClock, CollectionCoordinator, CollectionError, CollectionReport, ProviderSource,
};
use crate::database::connection::IndexStore;
use crate::providers::claude::ClaudeReader;
use crate::providers::codex::CodexReader;
use crate::sources::file_watcher::WatchRoot;
use crate::sources::provider_roots::resolve_native_root;
use crate::sources::session_files::{discover_native_sources, DiscoveryLimits};
use crate::types::provider::Provider;
use crate::types::usage_summary::UsageSummary;

pub const DEFAULT_DISCOVERY_LIMITS: DiscoveryLimits = DiscoveryLimits::new(5, 50 * 1024 * 1024);

struct Runtime {
    coordinator: CollectionCoordinator<IndexStore>,
    profile_root: PathBuf,
    discovery_limits: DiscoveryLimits,
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
}

impl fmt::Display for RuntimeInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::ProfileUnavailable => "profile_unavailable",
            Self::DataDirectory => "data_directory",
            Self::DatabaseOpen => "database_open",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for RuntimeInitError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Unavailable,
    StatePoisoned,
    Collection(CollectionError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("unavailable"),
            Self::StatePoisoned => formatter.write_str("state_poisoned"),
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
        let [claude_discovery, codex_discovery] =
            discover_native_sources(&self.profile_root, self.discovery_limits);
        let claude_reader = ClaudeReader::default();
        let codex_reader = CodexReader::default();
        let sources = [
            ProviderSource::new(true, claude_discovery, &claude_reader),
            ProviderSource::new(true, codex_discovery, &codex_reader),
        ];
        self.coordinator.collect(&sources, clock)
    }

    fn watch_roots(&self) -> Vec<WatchRoot> {
        [Provider::Claude, Provider::Codex]
            .into_iter()
            .filter_map(|provider| {
                resolve_native_root(&self.profile_root, provider)
                    .ok()
                    .map(|root| WatchRoot::new(provider, root.filesystem_path().to_path_buf()))
            })
            .collect()
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

        Ok(Self {
            runtime: Arc::new(Mutex::new(Some(Runtime {
                coordinator: CollectionCoordinator::new(store),
                profile_root,
                discovery_limits,
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
