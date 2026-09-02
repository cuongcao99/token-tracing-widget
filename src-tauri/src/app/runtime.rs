//! Managed runtime for one-shot native provider collection.

use std::collections::{BTreeMap, BTreeSet};
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
use crate::types::trace_signal::{ProviderEvent, TraceLifecycle, TraceSignal};
use crate::types::usage_summary::UsageSummary;
use crate::types::widget_settings::WidgetSettingsSnapshot;

pub const DEFAULT_DISCOVERY_LIMITS: DiscoveryLimits =
    DiscoveryLimits::without_file_count(50 * 1024 * 1024);
const TRACE_ACTIVITY_TTL: Duration = Duration::from_secs(120);

struct Runtime {
    coordinator: CollectionCoordinator<IndexStore>,
    profile_root: PathBuf,
    discovery_limits: DiscoveryLimits,
    invalid_settings: Vec<Provider>,
    widget_settings: WidgetSettingsSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TraceTransition {
    Started { provider: Provider, first_run: bool },
    Stopped { provider: Provider, last_run: bool },
    Ignored,
}

impl TraceTransition {
    #[cfg(test)]
    fn is_start(self) -> bool {
        matches!(self, Self::Started { .. })
    }

    #[cfg(test)]
    fn is_last_stop(self) -> bool {
        matches!(self, Self::Stopped { last_run: true, .. })
    }

    #[cfg(test)]
    fn is_ignored(self) -> bool {
        matches!(self, Self::Ignored)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraceSignalResult {
    pub(crate) summary: UsageSummary,
    pub(crate) transition: TraceTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TraceRunKey {
    provider: Provider,
    session_id: Option<String>,
}

#[derive(Debug, Clone)]
struct TraceRun {
    turn_id: Option<String>,
    received_at: Instant,
    generation: u64,
}

#[derive(Debug, Default)]
struct TraceActivityState {
    active_runs: BTreeMap<TraceRunKey, TraceRun>,
    hooked_providers: BTreeSet<Provider>,
    next_generation: u64,
}

impl TraceActivityState {
    fn apply(&mut self, signal: &TraceSignal, received_at: Instant) -> TraceTransition {
        self.hooked_providers.insert(signal.provider);
        match signal.lifecycle {
            TraceLifecycle::StartOrContinue => {
                let key = TraceRunKey {
                    provider: signal.provider,
                    session_id: signal.opaque_session_id.clone(),
                };
                let first_run = !self.has_active_run(signal.provider);
                let generation = self.next_generation();
                match self.active_runs.get_mut(&key) {
                    Some(run) => {
                        run.turn_id = signal.opaque_turn_id.clone();
                        run.received_at = received_at;
                        run.generation = generation;
                    }
                    None => {
                        self.active_runs.insert(
                            key,
                            TraceRun {
                                turn_id: signal.opaque_turn_id.clone(),
                                received_at,
                                generation,
                            },
                        );
                    }
                }
                TraceTransition::Started {
                    provider: signal.provider,
                    first_run,
                }
            }
            TraceLifecycle::Pause | TraceLifecycle::Stop => {
                let removed = if signal.provider_event == ProviderEvent::SessionEnd
                    && signal.opaque_session_id.is_none()
                {
                    let keys = self
                        .active_runs
                        .keys()
                        .filter(|key| key.provider == signal.provider)
                        .cloned()
                        .collect::<Vec<_>>();
                    let removed = !keys.is_empty();
                    for key in keys {
                        self.active_runs.remove(&key);
                    }
                    removed
                } else {
                    self.stop_matching_run(signal)
                };

                if removed {
                    TraceTransition::Stopped {
                        provider: signal.provider,
                        last_run: !self.has_active_run(signal.provider),
                    }
                } else {
                    TraceTransition::Ignored
                }
            }
        }
    }

    fn stop_matching_run(&mut self, signal: &TraceSignal) -> bool {
        let key = TraceRunKey {
            provider: signal.provider,
            session_id: signal.opaque_session_id.clone(),
        };
        let Some(run) = self.active_runs.get(&key) else {
            return false;
        };
        if signal
            .opaque_turn_id
            .as_deref()
            .zip(run.turn_id.as_deref())
            .is_some_and(|(incoming, current)| incoming != current)
        {
            return false;
        }
        self.active_runs.remove(&key).is_some()
    }

    fn next_generation(&mut self) -> u64 {
        self.next_generation = self.next_generation.saturating_add(1);
        self.next_generation
    }

    fn has_active_run(&self, provider: Provider) -> bool {
        self.active_runs.keys().any(|key| key.provider == provider)
    }

    fn latest_active_provider(&self) -> Option<Provider> {
        self.active_runs
            .iter()
            .max_by(|(_, left), (_, right)| {
                left.received_at
                    .cmp(&right.received_at)
                    .then_with(|| left.generation.cmp(&right.generation))
            })
            .map(|(key, _)| key.provider)
    }

    fn expire(&mut self, now: Instant) -> BTreeSet<Provider> {
        let expired = self
            .active_runs
            .iter()
            .filter(|(_, run)| now.saturating_duration_since(run.received_at) >= TRACE_ACTIVITY_TTL)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        let providers = expired
            .iter()
            .map(|key| key.provider)
            .collect::<BTreeSet<_>>();
        for key in expired {
            self.active_runs.remove(&key);
        }
        providers
    }
}

#[derive(Clone)]
pub struct AppState {
    runtime: Arc<Mutex<Option<Runtime>>>,
    source_configs: Arc<Mutex<SourceConfigSet>>,
    base_summary: Arc<Mutex<UsageSummary>>,
    trace_activity: Arc<Mutex<TraceActivityState>>,
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
                resolve_configured_root(&self.profile_root, config)
                    .ok()
                    .map(|root| WatchRoot::new(provider, root.filesystem_path().to_path_buf()))
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

fn compose_summary(
    mut summary: UsageSummary,
    activity: &TraceActivityState,
    source_configs: &SourceConfigSet,
) -> UsageSummary {
    if summary.state == crate::UsageState::Stale {
        return summary;
    }

    for provider_summary in &mut summary.providers {
        if source_configs.is_enabled(provider_summary.provider)
            && activity
                .hooked_providers
                .contains(&provider_summary.provider)
        {
            provider_summary.state = if activity.has_active_run(provider_summary.provider) {
                crate::UsageState::Active
            } else {
                crate::UsageState::Idle
            };
        }
    }

    if let Some(provider) = activity.latest_active_provider() {
        summary.state = crate::UsageState::Active;
        summary.provider = Some(provider.display_name().to_owned());
        for provider_summary in &mut summary.providers {
            if provider_summary.provider == provider {
                provider_summary.state = crate::UsageState::Active;
            }
        }
        return summary;
    }

    let global_provider_is_hooked = summary.provider.as_deref().is_some_and(|provider| {
        activity
            .hooked_providers
            .iter()
            .any(|hooked| hooked.display_name() == provider)
    });
    if global_provider_is_hooked {
        if let Some(provider_summary) = summary
            .providers
            .iter()
            .find(|provider_summary| provider_summary.state == crate::UsageState::Active)
        {
            summary.state = crate::UsageState::Active;
            summary.provider = Some(provider_summary.provider.display_name().to_owned());
        } else if summary.state == crate::UsageState::Active {
            summary.state = crate::UsageState::Idle;
            summary.provider = None;
        }
    }

    summary
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
            trace_activity: Arc::new(Mutex::new(TraceActivityState::default())),
            fallback_summary: UsageSummary::unavailable(),
        })
    }

    pub fn unavailable() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(None)),
            source_configs: Arc::new(Mutex::new(SourceConfigSet::defaults())),
            base_summary: Arc::new(Mutex::new(UsageSummary::unavailable())),
            trace_activity: Arc::new(Mutex::new(TraceActivityState::default())),
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

    pub(crate) fn apply_trace_signal(
        &self,
        signal: &TraceSignal,
        received_at: Instant,
    ) -> Result<TraceSignalResult, RuntimeError> {
        let enabled = self
            .source_configs
            .lock()
            .map_err(|_| RuntimeError::StatePoisoned)?
            .is_enabled(signal.provider);
        if !enabled {
            return Ok(TraceSignalResult {
                summary: self.summary(),
                transition: TraceTransition::Ignored,
            });
        }

        let transition = {
            let mut activity = self
                .trace_activity
                .lock()
                .map_err(|_| RuntimeError::StatePoisoned)?;
            let transition = activity.apply(signal, received_at);
            activity.expire(Instant::now());
            let transition = match transition {
                TraceTransition::Started { provider, .. } if !activity.has_active_run(provider) => {
                    TraceTransition::Ignored
                }
                transition => transition,
            };
            transition
        };
        Ok(TraceSignalResult {
            summary: self.summary(),
            transition,
        })
    }

    pub(crate) fn next_trace_expiry(&self) -> Option<Instant> {
        self.trace_activity.lock().ok().and_then(|activity| {
            activity
                .active_runs
                .values()
                .map(|run| run.received_at + TRACE_ACTIVITY_TTL)
                .min()
        })
    }

    pub(crate) fn expire_trace_activity(
        &self,
        now: Instant,
    ) -> Result<(UsageSummary, Vec<TraceTransition>), RuntimeError> {
        let transitions = {
            let mut activity = self
                .trace_activity
                .lock()
                .map_err(|_| RuntimeError::StatePoisoned)?;
            activity
                .expire(now)
                .into_iter()
                .map(|provider| TraceTransition::Stopped {
                    provider,
                    last_run: !activity.has_active_run(provider),
                })
                .collect()
        };
        Ok((self.summary(), transitions))
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
        let Ok(activity) = self.trace_activity.lock() else {
            return base_summary.clone();
        };
        let Ok(source_configs) = self.source_configs.lock() else {
            return base_summary.clone();
        };
        compose_summary(base_summary.clone(), &activity, &source_configs)
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
        signal_for(provider, lifecycle, event, "session-1", "turn-1")
    }

    fn signal_for(
        provider: Provider,
        lifecycle: TraceLifecycle,
        event: ProviderEvent,
        session_id: &str,
        turn_id: &str,
    ) -> TraceSignal {
        TraceSignal {
            schema_version: crate::types::trace_signal::TRACE_SIGNAL_SCHEMA_VERSION,
            provider,
            lifecycle,
            provider_event: event,
            observed_at: "2026-09-01T10:00:00Z".to_owned(),
            opaque_session_id: Some(session_id.to_owned()),
            opaque_turn_id: Some(turn_id.to_owned()),
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

        assert_eq!(active.summary.state, UsageState::Active);
        assert_eq!(active.summary.provider.as_deref(), Some("Claude Code"));
        assert_eq!(active.summary.today_tokens, 0);
        assert!(active.summary.current_session_tokens.is_none());
        assert!(active.summary.last_updated_at.is_none());
        assert_eq!(
            active
                .summary
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
        assert_eq!(unchanged.summary.state, UsageState::Active);
        assert_eq!(unchanged.summary.provider.as_deref(), Some("Claude Code"));

        let idle = state
            .apply_trace_signal(
                &signal(Provider::Claude, TraceLifecycle::Pause, ProviderEvent::Stop),
                Instant::now(),
            )
            .unwrap();
        assert_eq!(idle.summary.state, UsageState::Idle);
        assert!(idle.summary.provider.is_none());
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

        assert_eq!(expired.summary.state, UsageState::Idle);
        assert_eq!(expired.summary.today_tokens, 0);
        assert!(expired.summary.provider.is_none());
    }

    #[test]
    fn two_sessions_share_active_provider_until_both_stop() {
        let (state, _profile, _database) = state_with_native_roots();
        let now = Instant::now();

        let first = state
            .apply_trace_signal(
                &signal_for(
                    Provider::Claude,
                    TraceLifecycle::StartOrContinue,
                    ProviderEvent::UserPromptSubmit,
                    "session-a",
                    "turn-a",
                ),
                now,
            )
            .unwrap();
        assert_eq!(first.summary.state, UsageState::Active);
        assert!(first.transition.is_start());

        let second = state
            .apply_trace_signal(
                &signal_for(
                    Provider::Claude,
                    TraceLifecycle::StartOrContinue,
                    ProviderEvent::UserPromptSubmit,
                    "session-b",
                    "turn-b",
                ),
                now,
            )
            .unwrap();
        assert!(second.summary.state == UsageState::Active);

        let after_first_stop = state
            .apply_trace_signal(
                &signal_for(
                    Provider::Claude,
                    TraceLifecycle::Stop,
                    ProviderEvent::Stop,
                    "session-a",
                    "turn-a",
                ),
                now,
            )
            .unwrap();
        assert_eq!(after_first_stop.summary.state, UsageState::Active);
        assert!(!after_first_stop.transition.is_last_stop());

        let after_second_stop = state
            .apply_trace_signal(
                &signal_for(
                    Provider::Claude,
                    TraceLifecycle::Stop,
                    ProviderEvent::Stop,
                    "session-b",
                    "turn-b",
                ),
                now,
            )
            .unwrap();
        assert_eq!(after_second_stop.summary.state, UsageState::Idle);
        assert!(after_second_stop.transition.is_last_stop());
    }

    #[test]
    fn stale_stop_does_not_end_newer_turn_generation() {
        let (state, _profile, _database) = state_with_native_roots();
        let now = Instant::now();

        state
            .apply_trace_signal(
                &signal_for(
                    Provider::Codex,
                    TraceLifecycle::StartOrContinue,
                    ProviderEvent::UserPromptSubmit,
                    "session-a",
                    "turn-1",
                ),
                now,
            )
            .unwrap();
        state
            .apply_trace_signal(
                &signal_for(
                    Provider::Codex,
                    TraceLifecycle::StartOrContinue,
                    ProviderEvent::UserPromptSubmit,
                    "session-a",
                    "turn-2",
                ),
                now,
            )
            .unwrap();

        let stale = state
            .apply_trace_signal(
                &signal_for(
                    Provider::Codex,
                    TraceLifecycle::Stop,
                    ProviderEvent::Stop,
                    "session-a",
                    "turn-1",
                ),
                now,
            )
            .unwrap();

        assert!(stale.transition.is_ignored());
        assert_eq!(stale.summary.state, UsageState::Active);
    }

    #[test]
    fn lifecycle_signal_does_not_wait_for_collection_runtime_lock() {
        let (state, _profile, _database) = state_with_native_roots();
        let runtime_guard = state
            .runtime
            .lock()
            .expect("runtime lock should be available");
        let (sender, receiver) = std::sync::mpsc::channel();
        let signal_state = state.clone();
        let worker = std::thread::spawn(move || {
            let result = signal_state
                .apply_trace_signal(
                    &signal(
                        Provider::Claude,
                        TraceLifecycle::StartOrContinue,
                        ProviderEvent::UserPromptSubmit,
                    ),
                    Instant::now(),
                )
                .expect("lifecycle signal should be accepted");
            sender
                .send(result.summary.state)
                .expect("test receiver should remain connected");
        });

        assert_eq!(
            receiver
                .recv_timeout(Duration::from_millis(100))
                .expect("lifecycle signal should not wait for collection"),
            UsageState::Active
        );
        drop(runtime_guard);
        worker.join().expect("lifecycle worker should finish");
    }

    #[test]
    fn stop_forces_idle_even_when_token_event_is_recent() {
        let (state, profile, _database) = state_with_native_roots();
        std::fs::write(
            profile.path().join(r".claude\projects\session.jsonl"),
            r#"{"message":{"id":"event-1","type":"message","usage":{"input_tokens":10,"output_tokens":5}},"sessionId":"session-a","timestamp":"2026-09-01T10:00:00Z"}
"#,
        )
        .unwrap();
        state
            .collect_once(&FixedClock::new("2026-09-01T10:00:01Z", "2026-09-01"))
            .unwrap();

        state
            .apply_trace_signal(
                &signal_for(
                    Provider::Claude,
                    TraceLifecycle::StartOrContinue,
                    ProviderEvent::UserPromptSubmit,
                    "session-a",
                    "turn-a",
                ),
                Instant::now(),
            )
            .unwrap();
        let stopped = state
            .apply_trace_signal(
                &signal_for(
                    Provider::Claude,
                    TraceLifecycle::Stop,
                    ProviderEvent::Stop,
                    "session-a",
                    "turn-a",
                ),
                Instant::now(),
            )
            .unwrap();

        assert_eq!(stopped.summary.state, UsageState::Idle);
        assert_eq!(stopped.summary.today_tokens, 15);
        assert_eq!(
            stopped.summary.last_updated_at.as_deref(),
            Some("2026-09-01T10:00:00Z")
        );

        let flushed = state
            .collect_once(&FixedClock::new("2026-09-01T10:00:02Z", "2026-09-01"))
            .expect("final flush should complete");
        assert_eq!(flushed.summary.state, UsageState::Idle);
        assert_eq!(flushed.summary.today_tokens, 15);
    }
}
