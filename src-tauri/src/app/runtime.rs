//! Managed runtime for one-shot native provider collection.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
use crate::types::trace_signal::{TraceLifecycle, TraceSignal};
use crate::types::usage_summary::UsageSummary;
use crate::types::widget_settings::WidgetSettingsSnapshot;

pub const DEFAULT_DISCOVERY_LIMITS: DiscoveryLimits =
    DiscoveryLimits::without_file_count(50 * 1024 * 1024);
const TRACE_ACTIVITY_TTL: Duration = Duration::from_secs(120);

struct Runtime {
    coordinator: CollectionCoordinator<IndexStore>,
    profile_root: PathBuf,
    discovery_limits: DiscoveryLimits,
    source_configs: SourceConfigSet,
    invalid_settings: Vec<Provider>,
    widget_settings: WidgetSettingsSnapshot,
    trace_activity: Option<TraceActivity>,
}

#[derive(Debug, Clone, Copy)]
struct TraceActivity {
    provider: Provider,
    received_at: Instant,
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
        let mut report = self.coordinator.collect(&sources, clock)?;
        self.invalid_settings.clear();
        self.expire_trace_activity(Instant::now());
        report.summary = self.summary_with_trace(report.summary);
        Ok(report)
    }

    fn apply_trace_signal(&mut self, signal: &TraceSignal, received_at: Instant) -> UsageSummary {
        if self.source_configs.is_enabled(signal.provider) {
            match signal.lifecycle {
                TraceLifecycle::StartOrContinue => {
                    self.trace_activity = Some(TraceActivity {
                        provider: signal.provider,
                        received_at,
                    });
                }
                TraceLifecycle::Pause | TraceLifecycle::Stop => {
                    if self
                        .trace_activity
                        .is_some_and(|activity| activity.provider == signal.provider)
                    {
                        self.trace_activity = None;
                    }
                }
            }
        }
        self.expire_trace_activity(received_at);
        self.summary()
    }

    fn summary(&mut self) -> UsageSummary {
        self.expire_trace_activity(Instant::now());
        self.summary_with_trace(self.coordinator.last_summary().clone())
    }

    fn summary_with_trace(&self, mut summary: UsageSummary) -> UsageSummary {
        let Some(activity) = self.trace_activity else {
            return summary;
        };
        if summary.state == crate::UsageState::Stale {
            return summary;
        }

        summary.state = crate::UsageState::Active;
        summary.provider = Some(activity.provider.display_name().to_owned());
        for provider in &mut summary.providers {
            if provider.provider == activity.provider {
                provider.state = crate::UsageState::Active;
            }
        }
        summary
    }

    fn expire_trace_activity(&mut self, now: Instant) {
        if self
            .trace_activity
            .is_some_and(|activity| now.duration_since(activity.received_at) > TRACE_ACTIVITY_TTL)
        {
            self.trace_activity = None;
        }
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
                trace_activity: None,
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

    pub(crate) fn apply_trace_signal(
        &self,
        signal: &TraceSignal,
        received_at: Instant,
    ) -> Result<UsageSummary, RuntimeError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?;
        runtime
            .as_mut()
            .ok_or(RuntimeError::Unavailable)
            .map(|runtime| runtime.apply_trace_signal(signal, received_at))
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
        let Ok(mut runtime) = self.runtime.lock() else {
            return self.fallback_summary.clone();
        };
        runtime
            .as_mut()
            .map(Runtime::summary)
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
    use std::time::{Duration, Instant};

    use super::DEFAULT_DISCOVERY_LIMITS;
    use crate::collection::FixedClock;
    use crate::sources::session_files::DiscoveryLimits;
    use crate::types::provider::Provider;
    use crate::types::trace_signal::{ProviderEvent, TraceLifecycle, TraceSignal};
    use crate::UsageState;

    #[test]
    fn default_discovery_does_not_cap_file_count() {
        assert_eq!(DEFAULT_DISCOVERY_LIMITS.max_files, usize::MAX);
    }

    fn state_with_native_roots() -> (super::AppState, tempfile::TempDir, tempfile::TempDir) {
        let profile = tempfile::tempdir().expect("profile should be created");
        std::fs::create_dir_all(profile.path().join(r".claude\projects"))
            .expect("Claude root should be created");
        std::fs::create_dir_all(profile.path().join(r".codex\sessions"))
            .expect("Codex root should be created");
        let database = tempfile::tempdir().expect("database directory should be created");
        let state = super::AppState::from_paths(
            profile.path().to_path_buf(),
            &database.path().join("index.sqlite"),
            DiscoveryLimits::new(10, 10_000),
        )
        .expect("runtime should open");
        state
            .collect_once(&FixedClock::new("2026-09-01T10:00:00Z", "2026-09-01"))
            .expect("initial collection should complete");
        (state, profile, database)
    }

    fn signal(provider: Provider, lifecycle: TraceLifecycle, event: ProviderEvent) -> TraceSignal {
        TraceSignal {
            schema_version: crate::types::trace_signal::TRACE_SIGNAL_SCHEMA_VERSION,
            provider,
            lifecycle,
            provider_event: event,
            observed_at: "2026-09-01T10:00:00Z".to_owned(),
            opaque_session_id: Some("session-1".to_owned()),
            opaque_turn_id: Some("turn-1".to_owned()),
            sequence: None,
        }
    }

    #[test]
    fn hook_start_publishes_transient_active_state_without_tokens() {
        let (state, _profile, _database) = state_with_native_roots();
        let active = state
            .apply_trace_signal(
                &signal(
                    Provider::Claude,
                    TraceLifecycle::StartOrContinue,
                    ProviderEvent::UserPromptSubmit,
                ),
                Instant::now(),
            )
            .expect("runtime should accept hook signal");

        assert_eq!(active.state, UsageState::Active);
        assert_eq!(active.provider.as_deref(), Some("Claude Code"));
        assert_eq!(active.today_tokens, 0);
        assert!(active.current_session_tokens.is_none());
        assert_eq!(
            active
                .providers
                .iter()
                .find(|provider| provider.provider == Provider::Claude)
                .unwrap()
                .state,
            UsageState::Active
        );
    }

    #[test]
    fn hook_pause_only_clears_the_matching_provider_hint() {
        let (state, _profile, _database) = state_with_native_roots();
        state
            .apply_trace_signal(
                &signal(
                    Provider::Claude,
                    TraceLifecycle::StartOrContinue,
                    ProviderEvent::UserPromptSubmit,
                ),
                Instant::now(),
            )
            .unwrap();

        let unchanged = state
            .apply_trace_signal(
                &signal(Provider::Codex, TraceLifecycle::Pause, ProviderEvent::Stop),
                Instant::now(),
            )
            .unwrap();
        assert_eq!(unchanged.state, UsageState::Active);
        assert_eq!(unchanged.provider.as_deref(), Some("Claude Code"));

        let idle = state
            .apply_trace_signal(
                &signal(Provider::Claude, TraceLifecycle::Pause, ProviderEvent::Stop),
                Instant::now(),
            )
            .unwrap();
        assert_eq!(idle.state, UsageState::Idle);
        assert!(idle.provider.is_none());
    }

    #[test]
    fn hook_hint_expires_without_affecting_restart_safe_totals() {
        let (state, _profile, _database) = state_with_native_roots();
        let old = Instant::now()
            .checked_sub(Duration::from_secs(121))
            .expect("test instant should support subtraction");
        let expired = state
            .apply_trace_signal(
                &signal(
                    Provider::Claude,
                    TraceLifecycle::StartOrContinue,
                    ProviderEvent::UserPromptSubmit,
                ),
                old,
            )
            .unwrap();

        assert_eq!(expired.state, UsageState::Idle);
        assert_eq!(expired.today_tokens, 0);
        assert!(expired.provider.is_none());
    }
}
