use std::collections::BTreeSet;
use std::fmt;

use crate::providers::provider_adapter::ProviderAdapter;
use crate::sources::session_files::DiscoveryResult;
use crate::types::provider::Provider;
use crate::types::source_health::SourceHealth;
use crate::types::usage_summary::UsageSummary;
use crate::usage::summary::compute_summary;
use crate::utils::windows_time::{current_local_day, current_utc_timestamp, timestamp_local_day};

use super::persistence::{CollectionBatch, CollectionStore, CollectionStoreError, SourceUpdate};
use super::source_collection::{error_category, SourceCollectionResult};

pub struct ProviderSource<'a> {
    pub(super) enabled: bool,
    pub(super) configured_root: String,
    pub(super) settings_issue: bool,
    pub(super) discoveries: Vec<DiscoveryResult>,
    pub(super) adapter: &'a dyn ProviderAdapter,
}

impl<'a> ProviderSource<'a> {
    pub fn new(
        enabled: bool,
        discovery: DiscoveryResult,
        adapter: &'a dyn ProviderAdapter,
    ) -> Self {
        let configured_root = discovery.configured_root().to_owned();
        Self::with_discoveries(enabled, configured_root, false, vec![discovery], adapter)
    }

    pub fn with_configured_root(
        enabled: bool,
        configured_root: String,
        settings_issue: bool,
        discovery: DiscoveryResult,
        adapter: &'a dyn ProviderAdapter,
    ) -> Self {
        Self::with_discoveries(
            enabled,
            configured_root,
            settings_issue,
            vec![discovery],
            adapter,
        )
    }

    pub fn with_discoveries(
        enabled: bool,
        configured_root: String,
        settings_issue: bool,
        discoveries: Vec<DiscoveryResult>,
        adapter: &'a dyn ProviderAdapter,
    ) -> Self {
        Self {
            enabled,
            configured_root,
            settings_issue,
            discoveries,
            adapter,
        }
    }

    pub(super) fn provider(&self) -> Provider {
        self.discoveries
            .first()
            .expect("provider source should have at least one discovery")
            .provider()
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
        let summary = compute_summary(
            &rows,
            &source_health,
            &enabled_providers,
            clock.now(),
            clock.local_day(),
        );
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
