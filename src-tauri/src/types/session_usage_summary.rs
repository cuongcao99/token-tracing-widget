//! Privacy-safe current-day usage for one provider session.

use serde::Serialize;
use sha2::{Digest, Sha256};

pub const MAX_SESSION_LABEL_LENGTH: usize = 256;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionUsageState {
    Active,
    Idle,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionUsageSummary {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub state: SessionUsageState,
    pub today_tokens: u64,
}

pub fn normalize_session_name(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty()
        || value.len() > MAX_SESSION_LABEL_LENGTH
        || value.chars().any(char::is_control)
    {
        return None;
    }
    Some(value.to_owned())
}

pub fn safe_session_id(value: &str) -> String {
    if !value.is_empty()
        && value.len() <= MAX_SESSION_LABEL_LENGTH
        && !value.chars().any(char::is_control)
    {
        return value.to_owned();
    }

    let digest = Sha256::digest(value.as_bytes());
    format!("session-{digest:x}")
}
