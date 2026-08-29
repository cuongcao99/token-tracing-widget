use serde::Serialize;

mod app;
pub mod collection;
pub mod commands;
pub mod database;
pub mod providers;
pub mod sources;
pub mod types;
pub mod usage;
pub mod utils;

pub use types::source_health::SourceHealth;
pub use types::usage_summary::UsageSummary;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UsageState {
    Loading,
    Active,
    Idle,
    Unavailable,
    Stale,
}

#[tauri::command]
fn get_usage_summary() -> UsageSummary {
    UsageSummary::loading()
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
