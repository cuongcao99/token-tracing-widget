use std::collections::BTreeSet;
use std::fmt;

use crate::providers::provider_adapter::ProviderAdapter;
use crate::sources::session_files::DiscoveryResult;
use crate::types::provider::Provider;
use crate::types::source_health::SourceHealth;
use crate::types::usage_summary::UsageSummary;
use crate::usage::active_provider::{
    compute_active_provider, compute_current_session_tokens_for_local_day,
};
use crate::usage::daily_total::compute_today_total;
use crate::usage::provider_summary::compute_provider_summary;
use crate::usage::summary::SummaryRows;
use crate::utils::windows_time::{current_local_day, current_utc_timestamp, timestamp_local_day};
use crate::UsageState;

use super::persistence::{CollectionBatch, CollectionStore, CollectionStoreError, SourceUpdate};
use super::source_collection::{error_category, SourceCollectionResult};

pub struct ProviderSource<'a> {
    pub(super) enabled: bool,
    pub(super) configured_root: String,
    pub(super) settings_issue: bool,
    pub(super) discovery: DiscoveryResult,
    pub(super) adapter: &'a dyn ProviderAdapter,
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

    pub(super) fn provider(&self) -> Provider {
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
    pub has_pending_reads: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionError {
    Storage(CollectionStoreError),
}

impl fmt::Display for CollectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "storage:{error}"),
        }
    }
}

impl std::error::Error for CollectionError {}

impl From<CollectionStoreError> for CollectionError {
    fn from(error: CollectionStoreError) -> Self {
        Self::Storage(error)
    }
}

pub struct CollectionCoordinator<S> {
    pub(super) store: S,
    last_summary: UsageSummary,
}

impl<S: CollectionStore> CollectionCoordinator<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            last_summary: UsageSummary::loading(),
        }
    }

    pub(crate) fn store_mut(&mut self) -> &mut S {
        &mut self.store
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
        let mut allowed_codex_file_identities = BTreeSet::new();
        let mut has_pending_reads = false;
        for source in ordered {
            let SourceCollectionResult {
                events,
                checkpoints,
                health,
                diagnostics,
                session_key_updates,
                session_name_updates,
                rate_limit_updates,
                has_pending_reads: source_has_pending_reads,
                allowed_file_identities,
            } = match self.collect_source(source, clock.now(), clock.local_day()) {
                Ok(result) => result,
                Err(error) => {
                    self.last_summary = UsageSummary::stale_from(&self.last_summary);
                    return Err(error);
                }
            };
            has_pending_reads |= source_has_pending_reads;
            batch.events.extend(events);
            batch.checkpoints.extend(checkpoints);
            batch.session_key_updates.extend(session_key_updates);
            batch.session_name_updates.extend(session_name_updates);
            batch.rate_limit_updates.extend(rate_limit_updates);
            batch.diagnostics.extend(diagnostics);
            if source.provider() == Provider::Codex {
                allowed_codex_file_identities.extend(allowed_file_identities);
            }
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
        let mut rows = match self.store.query_events_for_summary("", clock.now()) {
            Ok(rows) => rows,
            Err(error) => {
                self.last_summary = UsageSummary::stale_from(&self.last_summary);
                return Err(CollectionError::Storage(error));
            }
        };
        rows.events.retain(|event| {
            event.provider != Provider::Codex
                || timestamp_local_day(&event.observed_at).as_deref() != Some(clock.local_day())
                || allowed_codex_file_identities.contains(&event.file_identity)
        });
        let summary = compute_summary(&rows, &source_health, &enabled_providers, clock);
        self.last_summary = summary.clone();

        Ok(CollectionReport {
            summary,
            accepted_event_count,
            source_health,
            has_pending_reads,
        })
    }

    pub fn last_summary(&self) -> &UsageSummary {
        &self.last_summary
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
    let providers = Provider::all()
        .iter()
        .copied()
        .map(|provider| {
            let health = source_health
                .iter()
                .find(|entry| entry.provider == provider);
            let rate_limits: Vec<_> = if enabled_providers.contains(&provider)
                && health.is_some_and(|health| {
                    matches!(health.state.as_str(), "detected" | "limited" | "malformed")
                }) {
                rows.rate_limits
                    .iter()
                    .filter(|entry| entry.provider == provider)
                    .map(|entry| entry.rate_limit)
                    .collect()
            } else {
                Vec::new()
            };
            compute_provider_summary(
                provider,
                &enabled_events,
                health,
                &rate_limits,
                clock.now(),
                clock.local_day(),
            )
        })
        .collect();

    UsageSummary {
        state,
        provider: active.provider,
        current_session_tokens: compute_current_session_tokens_for_local_day(
            &enabled_events,
            clock.now(),
            clock.local_day(),
        ),
        today_tokens: compute_today_total(&enabled_events, clock.local_day()),
        last_updated_at: active.last_updated_at,
        source_health: source_health.to_vec(),
        providers,
    }
}
