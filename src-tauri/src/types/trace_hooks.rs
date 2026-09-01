//! Frontend-safe state for optional provider lifecycle hooks.

use serde::Serialize;

use super::provider::Provider;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceHookState {
    NotInstalled,
    Configured,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceHookStatus {
    pub provider: Provider,
    pub state: TraceHookState,
    pub requires_trust: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceHooksSnapshot {
    pub providers: Vec<TraceHookStatus>,
}
