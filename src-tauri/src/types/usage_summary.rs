//! The aggregate returned to the overlay.

use serde::Serialize;

use crate::{SourceHealth, UsageState};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub state: UsageState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_session_tokens: Option<u64>,
    pub today_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated_at: Option<String>,
    pub source_health: Vec<SourceHealth>,
}

impl UsageSummary {
    pub fn loading() -> Self {
        Self {
            state: UsageState::Loading,
            provider: None,
            current_session_tokens: None,
            today_tokens: 0,
            last_updated_at: None,
            source_health: Vec::new(),
        }
    }

    pub fn unavailable() -> Self {
        Self {
            state: UsageState::Unavailable,
            provider: None,
            current_session_tokens: None,
            today_tokens: 0,
            last_updated_at: None,
            source_health: Vec::new(),
        }
    }

    pub fn stale_from(previous: &Self) -> Self {
        Self {
            state: UsageState::Stale,
            provider: previous.provider.clone(),
            current_session_tokens: previous.current_session_tokens,
            today_tokens: previous.today_tokens,
            last_updated_at: previous.last_updated_at.clone(),
            source_health: previous.source_health.clone(),
        }
    }
}
