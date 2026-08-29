use serde::Serialize;

mod app;
pub mod commands;
pub mod database;
pub mod providers;
pub mod sources;
pub mod types;
pub mod usage;
pub mod utils;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UsageState {
    Loading,
    Active,
    Idle,
    Unavailable,
    Stale,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealth {
    pub provider: String,
    pub state: String,
}

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

#[tauri::command]
fn get_usage_summary() -> UsageSummary {
    UsageSummary {
        state: UsageState::Loading,
        provider: None,
        current_session_tokens: None,
        today_tokens: 0,
        last_updated_at: None,
        source_health: Vec::new(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_usage_summary])
        .run(tauri::generate_context!())
        .expect("error while running token tracing widget");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_summary_contains_no_provider_data() {
        let summary = get_usage_summary();

        assert_eq!(summary.state, UsageState::Loading);
        assert_eq!(summary.today_tokens, 0);
        assert!(summary.provider.is_none());
        assert!(summary.current_session_tokens.is_none());
        assert!(summary.last_updated_at.is_none());
        assert!(summary.source_health.is_empty());

        let serialized = serde_json::to_value(&summary).expect("summary should serialize");
        let object = serialized
            .as_object()
            .expect("summary should serialize as an object");

        assert!(!object.contains_key("provider"));
        assert!(!object.contains_key("currentSessionTokens"));
        assert!(!object.contains_key("lastUpdatedAt"));
    }
}
