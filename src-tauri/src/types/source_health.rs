//! Sanitized health state for one configured source.

use serde::Serialize;

use super::provider::Provider;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealth {
    pub provider: Provider,
    pub state: String,
}

impl SourceHealth {
    pub fn new(provider: Provider, state: impl Into<String>) -> Self {
        Self {
            provider,
            state: state.into(),
        }
    }

    pub fn detected(provider: Provider) -> Self {
        Self::new(provider, "detected")
    }
}
