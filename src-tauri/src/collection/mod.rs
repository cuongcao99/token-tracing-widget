//! Collection inputs shared by the pure core and the SQLite boundary.

use std::fmt;

use crate::database::connection::IndexStore;
use crate::providers::provider_adapter::{ProviderAdapter, ProviderReadError};
use crate::sources::session_files::{DiscoveryResult, DiscoveryStatus};
use crate::sources::source_config::SourceConfig;
use crate::types::file_checkpoint::FileCheckpoint;
use crate::types::provider::Provider;
use crate::types::source_health::SourceHealth;
use crate::types::usage_event::UsageEvent;
use crate::types::usage_summary::UsageSummary;
use crate::types::widget_settings::WidgetSettingsSnapshot;
use crate::usage::active_provider::compute_active_provider;
use crate::usage::cumulative_delta::{convert_observations, DeltaConversionError};
use crate::usage::daily_total::compute_today_total;
use crate::usage::provider_summary::compute_provider_summary;
use crate::utils::windows_time::{current_local_day, current_utc_timestamp};
use crate::UsageState;

pub use crate::database::connection::{StorageError, SummaryRows};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionBatch {
    pub events: Vec<UsageEvent>,
    pub checkpoints: Vec<FileCheckpoint>,
    pub source_updates: Vec<SourceUpdate>,
    pub diagnostics: Vec<DiagnosticUpdate>,
}

impl CollectionBatch {
    pub fn new(events: Vec<UsageEvent>, checkpoints: Vec<FileCheckpoint>) -> Self {
        Self {
            events,
            checkpoints,
            source_updates: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUpdate {
    pub provider: Provider,
    pub configured_root: String,
    pub enabled: bool,
    pub health_state: String,
    pub last_error_category: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticUpdate {
    pub provider: Provider,
    pub category: String,
    pub occurrence_count: u64,
    pub last_occurred_at: String,
}

pub trait CollectionStore {
    fn load_checkpoint(&self, identity: &str) -> Result<Option<FileCheckpoint>, StorageError>;

    fn apply_batch(&mut self, batch: &CollectionBatch) -> Result<(), StorageError>;

    fn query_events_for_summary(
        &self,
        day_start: &str,
        now: &str,
    ) -> Result<SummaryRows, StorageError>;
}

impl CollectionStore for IndexStore {
    fn load_checkpoint(&self, identity: &str) -> Result<Option<FileCheckpoint>, StorageError> {
        IndexStore::load_checkpoint(self, identity)
    }

    fn apply_batch(&mut self, batch: &CollectionBatch) -> Result<(), StorageError> {
        IndexStore::apply_batch(self, batch)
    }

    fn query_events_for_summary(
        &self,
        day_start: &str,
        now: &str,
    ) -> Result<SummaryRows, StorageError> {
        IndexStore::query_events_for_summary(self, day_start, now)
    }
}

pub struct ProviderSource<'a> {
    enabled: bool,
    configured_root: String,
    settings_issue: bool,
    discovery: DiscoveryResult,
    adapter: &'a dyn ProviderAdapter,
}

impl<'a> ProviderSource<'a> {
    pub fn new(
        enabled: bool,
        discovery: DiscoveryResult,
        adapter: &'a dyn ProviderAdapter,
    ) -> Self {
        let configured_root = discovery.configured_root().to_owned();
        Self::with_configured_root(enabled, configured_root, false, discovery, adapter)
    }

    pub fn with_configured_root(
        enabled: bool,
        configured_root: String,
        settings_issue: bool,
        discovery: DiscoveryResult,
        adapter: &'a dyn ProviderAdapter,
    ) -> Self {
        Self {
            enabled,
            configured_root,
            settings_issue,
            discovery,
            adapter,
        }
    }

    fn provider(&self) -> Provider {
        self.discovery.provider()
    }
}

pub trait CollectionClock {
    fn now(&self) -> &str;

    fn local_day(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedClock {
    now: String,
    local_day: String,
}

impl FixedClock {
    pub fn new(now: &str, local_day: &str) -> Self {
        Self {
            now: now.to_owned(),
            local_day: local_day.to_owned(),
        }
    }
}

impl CollectionClock for FixedClock {
    fn now(&self) -> &str {
        &self.now
    }

    fn local_day(&self) -> &str {
        &self.local_day
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsClock {
    now_value: String,
    local_day_value: String,
}

impl WindowsClock {
    pub fn current() -> Self {
        Self {
            now_value: current_utc_timestamp(),
            local_day_value: current_local_day(),
        }
    }

    pub fn now(&self) -> &str {
        &self.now_value
    }

    pub fn local_day(&self) -> &str {
        &self.local_day_value
    }
}

impl CollectionClock for WindowsClock {
    fn now(&self) -> &str {
        self.now()
    }

    fn local_day(&self) -> &str {
        self.local_day()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionReport {
    pub summary: UsageSummary,
    pub accepted_event_count: usize,
    pub source_health: Vec<SourceHealth>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionError {
    Storage(StorageError),
}

impl fmt::Display for CollectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "storage:{error}"),
        }
    }
}

impl std::error::Error for CollectionError {}

impl From<StorageError> for CollectionError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

pub struct CollectionCoordinator<S> {
    store: S,
    last_summary: UsageSummary,
}

impl<S: CollectionStore> CollectionCoordinator<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            last_summary: UsageSummary::loading(),
        }
    }

    pub fn collect(
        &mut self,
        sources: &[ProviderSource<'_>],
        clock: &dyn CollectionClock,
    ) -> Result<CollectionReport, CollectionError> {
        let mut ordered: Vec<_> = sources.iter().collect();
        ordered.sort_by_key(|source| source.provider().as_str());
        let enabled_providers: Vec<_> = ordered
            .iter()
            .filter(|source| source.enabled)
            .map(|source| source.provider())
            .collect();

        let mut batch = CollectionBatch::new(Vec::new(), Vec::new());
        let mut source_health = Vec::with_capacity(ordered.len());
        for source in ordered {
            let (events, checkpoints, health, diagnostics) =
                match self.collect_source(source, clock.now()) {
                    Ok(result) => result,
                    Err(error) => {
                        self.last_summary = UsageSummary::stale_from(&self.last_summary);
                        return Err(error);
                    }
                };
            batch.events.extend(events);
            batch.checkpoints.extend(checkpoints);
            batch.diagnostics.extend(diagnostics);
            batch.source_updates.push(SourceUpdate {
                provider: source.provider(),
                configured_root: source.configured_root.clone(),
                enabled: source.enabled,
                health_state: health.state.clone(),
                last_error_category: error_category(&health.state),
                updated_at: clock.now().to_owned(),
            });
            source_health.push(health);
        }

        let accepted_event_count = batch.events.len();
        if let Err(error) = self.store.apply_batch(&batch) {
            self.last_summary = UsageSummary::stale_from(&self.last_summary);
            return Err(CollectionError::Storage(error));
        }
        let rows = match self.store.query_events_for_summary("", clock.now()) {
            Ok(rows) => rows,
            Err(error) => {
                self.last_summary = UsageSummary::stale_from(&self.last_summary);
                return Err(CollectionError::Storage(error));
            }
        };
        let summary = compute_summary(&rows, &source_health, &enabled_providers, clock);
        self.last_summary = summary.clone();

        Ok(CollectionReport {
            summary,
            accepted_event_count,
            source_health,
        })
    }

    pub fn last_summary(&self) -> &UsageSummary {
        &self.last_summary
    }

    fn collect_source(
        &self,
        source: &ProviderSource<'_>,
        now: &str,
    ) -> Result<
        (
            Vec<UsageEvent>,
            Vec<FileCheckpoint>,
            SourceHealth,
            Vec<DiagnosticUpdate>,
        ),
        CollectionError,
    > {
        let provider = source.provider();
        let mut diagnostics = Vec::new();
        if source.settings_issue {
            diagnostics.push(DiagnosticUpdate {
                provider,
                category: "invalid_settings".to_owned(),
                occurrence_count: 1,
                last_occurred_at: now.to_owned(),
            });
        }
        if !source.enabled {
            return Ok((
                Vec::new(),
                Vec::new(),
                SourceHealth::new(provider, "disabled"),
                diagnostics,
            ));
        }

        let mut health_state = discovery_state(source.discovery.status()).to_owned();
        let mut events = Vec::new();
        let mut checkpoints = Vec::new();
        for file in source.discovery.files() {
            let identity = file.opaque_identity(provider);
            let checkpoint = self
                .store
                .load_checkpoint(&identity)
                .map_err(CollectionError::Storage)?
                .filter(|checkpoint| checkpoint_can_resume(checkpoint, file, provider))
                .unwrap_or_else(|| FileCheckpoint::new(identity.clone(), provider));

            let result = match source
                .adapter
                .read_observations(file.filesystem_path(), checkpoint.byte_offset)
            {
                Ok(result) => result,
                Err(error) => {
                    let state = reader_error_state(error);
                    health_state = state.to_owned();
                    diagnostics.push(DiagnosticUpdate {
                        provider,
                        category: state.to_owned(),
                        occurrence_count: 1,
                        last_occurred_at: now.to_owned(),
                    });
                    continue;
                }
            };
            let delta = match convert_observations(&identity, &checkpoint, result.observations) {
                Ok(delta) => delta,
                Err(error) => {
                    let state = conversion_error_state(error);
                    health_state = state.to_owned();
                    diagnostics.push(DiagnosticUpdate {
                        provider,
                        category: state.to_owned(),
                        occurrence_count: 1,
                        last_occurred_at: now.to_owned(),
                    });
                    continue;
                }
            };
            let mut next_checkpoint = delta.next_checkpoint;
            next_checkpoint.byte_offset = result.next_offset;
            next_checkpoint.pending_offset = result.pending_offset;
            next_checkpoint =
                next_checkpoint.with_file_metadata(file.size_bytes(), file.modified_at_unix_ms());
            events.extend(delta.events);
            checkpoints.push(next_checkpoint);
        }

        if let Some(category) = error_category(&health_state) {
            if diagnostics.is_empty() {
                diagnostics.push(DiagnosticUpdate {
                    provider,
                    category,
                    occurrence_count: 1,
                    last_occurred_at: now.to_owned(),
                });
            }
        }

        Ok((
            events,
            checkpoints,
            SourceHealth::new(provider, health_state),
            diagnostics,
        ))
    }
}

impl CollectionCoordinator<IndexStore> {
    pub fn save_source_config(&mut self, config: &SourceConfig) -> Result<(), StorageError> {
        self.store.save_source_config(config)
    }

    pub fn save_widget_settings(
        &mut self,
        settings: &WidgetSettingsSnapshot,
    ) -> Result<(), StorageError> {
        self.store.save_widget_settings(settings)
    }
}

pub fn compute_summary(
    rows: &SummaryRows,
    source_health: &[SourceHealth],
    enabled_providers: &[Provider],
    clock: &dyn CollectionClock,
) -> UsageSummary {
    let enabled_events: Vec<_> = rows
        .events
        .iter()
        .filter(|event| enabled_providers.contains(&event.provider))
        .cloned()
        .collect();
    let active = compute_active_provider(&enabled_events, clock.now());
    let usable_source = source_health
        .iter()
        .any(|health| matches!(health.state.as_str(), "detected" | "limited" | "malformed"));
    let state = if active.state == UsageState::Active {
        UsageState::Active
    } else if usable_source {
        UsageState::Idle
    } else {
        UsageState::Unavailable
    };
    let providers = [Provider::Claude, Provider::Codex]
        .into_iter()
        .map(|provider| {
            let health = source_health
                .iter()
                .find(|entry| entry.provider == provider);
            compute_provider_summary(
                provider,
                &enabled_events,
                health,
                clock.now(),
                clock.local_day(),
            )
        })
        .collect();

    UsageSummary {
        state,
        provider: active.provider,
        current_session_tokens: active.current_session_tokens,
        today_tokens: compute_today_total(&enabled_events, clock.local_day()),
        last_updated_at: active.last_updated_at,
        source_health: source_health.to_vec(),
        providers,
    }
}

fn discovery_state(status: DiscoveryStatus) -> &'static str {
    match status {
        DiscoveryStatus::Disabled => "disabled",
        DiscoveryStatus::Detected => "detected",
        DiscoveryStatus::NotDetected => "not_detected",
        DiscoveryStatus::PermissionDenied => "permission_denied",
        DiscoveryStatus::InvalidRoot => "invalid_root",
        DiscoveryStatus::Unavailable => "unavailable",
        DiscoveryStatus::LimitReached => "limited",
    }
}

fn reader_error_state(error: ProviderReadError) -> &'static str {
    match error {
        ProviderReadError::Io => "unavailable",
        ProviderReadError::InvalidJson | ProviderReadError::InvalidRecord => "malformed",
        ProviderReadError::InvalidTokenCount => "malformed",
        ProviderReadError::RecordTooLarge => "limited",
    }
}

fn conversion_error_state(_error: DeltaConversionError) -> &'static str {
    "malformed"
}

fn error_category(state: &str) -> Option<String> {
    matches!(
        state,
        "permission_denied"
            | "invalid_root"
            | "unavailable"
            | "limited"
            | "malformed"
            | "unsupported_format"
    )
    .then(|| state.to_owned())
}

fn checkpoint_can_resume(
    checkpoint: &FileCheckpoint,
    file: &crate::sources::session_files::DiscoveredSessionFile,
    provider: Provider,
) -> bool {
    checkpoint.provider == provider
        && checkpoint.size_bytes <= file.size_bytes()
        && checkpoint.byte_offset <= file.size_bytes()
        && !(checkpoint.size_bytes == file.size_bytes()
            && checkpoint.modified_at_unix_ms != 0
            && checkpoint.modified_at_unix_ms != file.modified_at_unix_ms())
}
