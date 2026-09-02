//! One privacy-safe token observation emitted by a provider reader.

use super::provider::Provider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterKind {
    Incremental,
    Cumulative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenObservation {
    pub provider: Provider,
    pub source_session_key: Option<String>,
    pub session_name: Option<String>,
    pub source_event_key: Option<String>,
    pub observed_at: String,
    pub counter_kind: CounterKind,
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: u64,
}
