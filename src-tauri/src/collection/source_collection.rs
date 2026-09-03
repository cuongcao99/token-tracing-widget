use std::collections::BTreeSet;

use crate::providers::provider_adapter::{ProviderReadError, MAX_SOURCE_BYTES_PER_ATTEMPT};
use crate::sources::session_files::DiscoveryStatus;
use crate::types::file_checkpoint::FileCheckpoint;
use crate::types::provider::Provider;
use crate::types::source_health::SourceHealth;
use crate::usage::cumulative_delta::{convert_observations, DeltaConversionError};

use super::coordinator::CollectionError;
use super::coordinator::{CollectionCoordinator, ProviderSource};
use super::persistence::{
    CollectionStore, DiagnosticUpdate, RateLimitUpdate, SessionKeyUpdate, SessionNameUpdate,
};

pub(super) struct SourceCollectionResult {
    pub(super) events: Vec<crate::types::usage_event::UsageEvent>,
    pub(super) checkpoints: Vec<FileCheckpoint>,
    pub(super) health: SourceHealth,
    pub(super) diagnostics: Vec<DiagnosticUpdate>,
    pub(super) session_key_updates: Vec<SessionKeyUpdate>,
    pub(super) session_name_updates: Vec<SessionNameUpdate>,
    pub(super) rate_limit_updates: Vec<RateLimitUpdate>,
    pub(super) has_pending_reads: bool,
    pub(super) allowed_file_identities: BTreeSet<String>,
}

impl<S: CollectionStore> CollectionCoordinator<S> {
    pub(super) fn collect_source(
        &self,
        source: &ProviderSource<'_>,
        now: &str,
        local_day: &str,
    ) -> Result<SourceCollectionResult, CollectionError> {
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
            return Ok(SourceCollectionResult {
                events: Vec::new(),
                checkpoints: Vec::new(),
                health: SourceHealth::new(provider, "disabled"),
                diagnostics,
                session_key_updates: Vec::new(),
                session_name_updates: Vec::new(),
                rate_limit_updates: Vec::new(),
                has_pending_reads: false,
                allowed_file_identities: BTreeSet::new(),
            });
        }

        let mut health_state = discovery_state(source.discovery.status()).to_owned();
        let mut events = Vec::new();
        let mut checkpoints = Vec::new();
        let mut session_key_updates = Vec::new();
        let mut session_name_updates = Vec::new();
        let mut rate_limit_updates = Vec::new();
        let mut allowed_file_identities = BTreeSet::new();
        let mut has_pending_reads = false;
        let mut remaining_source_bytes = MAX_SOURCE_BYTES_PER_ATTEMPT;
        for file in source.discovery.files() {
            let is_allowed_file = source
                .adapter
                .should_read_file(file.filesystem_path(), local_day);
            if !is_allowed_file {
                rate_limit_updates.extend(
                    source
                        .adapter
                        .read_rate_limits(file.filesystem_path())
                        .into_iter()
                        .map(|snapshot| RateLimitUpdate { provider, snapshot }),
                );
                continue;
            }
            let identity = file.opaque_identity(provider);
            allowed_file_identities.insert(identity.clone());
            if remaining_source_bytes == 0 {
                has_pending_reads = true;
                rate_limit_updates.extend(
                    source
                        .adapter
                        .read_rate_limits(file.filesystem_path())
                        .into_iter()
                        .map(|snapshot| RateLimitUpdate { provider, snapshot }),
                );
                continue;
            }
            let checkpoint = self
                .store
                .load_checkpoint(&identity)
                .map_err(CollectionError::Storage)?
                .filter(|checkpoint| checkpoint_can_resume(checkpoint, file, provider))
                .unwrap_or_else(|| FileCheckpoint::new(identity.clone(), provider));

            let result = match source.adapter.read_observations(
                file.filesystem_path(),
                checkpoint.byte_offset,
                remaining_source_bytes,
            ) {
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
            if let Some(session_key) = result.session_key.as_ref() {
                session_key_updates.push(SessionKeyUpdate {
                    provider,
                    file_identity: identity.clone(),
                    session_key: session_key.clone(),
                });
            }
            if let (Some(name), Some(updated_at)) = (
                result.session_name.as_ref(),
                result.session_name_updated_at.as_ref(),
            ) {
                session_name_updates.push(SessionNameUpdate {
                    provider,
                    session_key: result
                        .session_key
                        .clone()
                        .unwrap_or_else(|| identity.clone()),
                    name: name.clone(),
                    updated_at: updated_at.clone(),
                });
            }
            rate_limit_updates.extend(
                result
                    .rate_limits
                    .iter()
                    .cloned()
                    .map(|snapshot| RateLimitUpdate { provider, snapshot }),
            );
            remaining_source_bytes = remaining_source_bytes.saturating_sub(result.bytes_read);
            if result.pending_offset.is_none() && result.next_offset < file.size_bytes() {
                has_pending_reads = true;
            }
            if result.skipped_oversized_records > 0 {
                health_state = "limited".to_owned();
                diagnostics.push(DiagnosticUpdate {
                    provider,
                    category: "limited".to_owned(),
                    occurrence_count: 1,
                    last_occurred_at: now.to_owned(),
                });
            }
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

        Ok(SourceCollectionResult {
            events,
            checkpoints,
            health: SourceHealth::new(provider, health_state),
            diagnostics,
            session_key_updates,
            session_name_updates,
            rate_limit_updates,
            has_pending_reads,
            allowed_file_identities,
        })
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

pub(super) fn error_category(state: &str) -> Option<String> {
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
