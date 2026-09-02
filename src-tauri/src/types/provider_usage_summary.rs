//! Privacy-safe usage totals for one supported Provider.

use serde::Serialize;

use crate::UsageState;

use super::provider::Provider;
use super::session_usage_summary::SessionUsageSummary;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageSummary {
    pub provider: Provider,
    pub state: UsageState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_session_tokens: Option<u64>,
    pub today_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated_at: Option<String>,
    pub sessions: Vec<SessionUsageSummary>,
}

impl ProviderUsageSummary {
    pub fn new(
        provider: Provider,
        state: UsageState,
        current_session_tokens: Option<u64>,
        today_tokens: u64,
        last_updated_at: Option<String>,
        sessions: Vec<SessionUsageSummary>,
    ) -> Self {
        Self {
            provider,
            state,
            current_session_tokens,
            today_tokens,
            last_updated_at,
            sessions,
        }
    }
}
