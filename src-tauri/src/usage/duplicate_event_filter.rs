//! Rejecting usage events already accepted by the index.

use sha2::{Digest, Sha256};

use crate::types::provider::Provider;
use crate::types::token_observation::{CounterKind, TokenObservation};

pub fn event_id(
    provider: Provider,
    file_identity: &str,
    observation: &TokenObservation,
    source_position: u64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(provider.as_str().as_bytes());
    hasher.update([0]);

    if let Some(session_key) = observation.source_session_key.as_deref() {
        hasher.update(b"session:");
        hasher.update(session_key.as_bytes());
        hasher.update([0]);
    }

    if let Some(source_event_key) = observation.source_event_key.as_deref() {
        hasher.update(b"event:");
        hasher.update(source_event_key.as_bytes());
    } else {
        hasher.update(b"file:");
        hasher.update(file_identity.as_bytes());
        hasher.update([0]);
        hasher.update(source_position.to_le_bytes());
        hasher.update([0]);
        hasher.update(counter_kind_name(observation.counter_kind).as_bytes());
    }

    format!("{:x}", hasher.finalize())
}

pub fn effective_session_key(observation: &TokenObservation, file_identity: &str) -> String {
    observation
        .source_session_key
        .clone()
        .unwrap_or_else(|| file_identity.to_owned())
}

fn counter_kind_name(kind: CounterKind) -> &'static str {
    match kind {
        CounterKind::Incremental => "incremental",
        CounterKind::Cumulative => "cumulative",
    }
}
