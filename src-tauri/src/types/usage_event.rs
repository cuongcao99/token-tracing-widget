//! A deduplicated token usage event accepted by the totals pipeline.

use super::provider::Provider;
use super::token_observation::{CounterKind, TokenObservation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    pub event_id: String,
    pub provider: Provider,
    pub file_identity: String,
    pub session_key: String,
    pub source_position: u64,
    pub observed_at: String,
    pub counter_kind: CounterKind,
    pub monotonic_segment: u64,
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: u64,
}

impl UsageEvent {
    pub fn from_delta(
        event_id: String,
        file_identity: String,
        source_position: u64,
        observation: &TokenObservation,
        session_key: String,
        monotonic_segment: u64,
        input_tokens: Option<u64>,
        cached_input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        total_tokens: u64,
    ) -> Self {
        Self {
            event_id,
            provider: observation.provider,
            file_identity,
            session_key,
            source_position,
            observed_at: observation.observed_at.clone(),
            counter_kind: observation.counter_kind,
            monotonic_segment,
            input_tokens,
            cached_input_tokens,
            output_tokens,
            total_tokens,
        }
    }
}
