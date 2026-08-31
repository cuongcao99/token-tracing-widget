//! Returning the current privacy-safe usage summary.

use std::fmt;

use tauri::{AppHandle, Emitter, State};

use crate::{AppState, UsageSummary};

pub const USAGE_SUMMARY_CHANGED_EVENT: &str = "usage-summary-changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryEventError {
    Emit,
}

impl fmt::Display for SummaryEventError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("emit")
    }
}

impl std::error::Error for SummaryEventError {}

#[tauri::command]
pub fn get_usage_summary(state: State<'_, AppState>) -> UsageSummary {
    state.summary()
}

pub fn emit_usage_summary(
    app: &AppHandle,
    summary: &UsageSummary,
) -> Result<(), SummaryEventError> {
    app.emit(USAGE_SUMMARY_CHANGED_EVENT, summary)
        .map_err(|_| SummaryEventError::Emit)
}

#[cfg(test)]
mod tests {
    use super::USAGE_SUMMARY_CHANGED_EVENT;
    use crate::types::provider::Provider;
    use crate::types::provider_usage_summary::ProviderUsageSummary;
    use crate::{SourceHealth, UsageState, UsageSummary};

    #[test]
    fn summary_contract_contains_only_allowed_wire_fields() {
        let summary = UsageSummary {
            state: UsageState::Active,
            provider: Some("Claude Code".to_owned()),
            current_session_tokens: Some(20),
            today_tokens: 20,
            last_updated_at: Some("2026-01-01T00:00:00Z".to_owned()),
            source_health: vec![SourceHealth::detected(Provider::Claude)],
            providers: vec![ProviderUsageSummary::new(
                Provider::Claude,
                UsageState::Active,
                Some(20),
                20,
                Some("2026-01-01T00:00:00Z".to_owned()),
            )],
        };
        let object = serde_json::to_value(summary)
            .expect("summary should serialize")
            .as_object()
            .cloned()
            .expect("summary should be an object");

        assert_eq!(
            object.keys().map(String::as_str).collect::<Vec<_>>(),
            vec![
                "currentSessionTokens",
                "lastUpdatedAt",
                "provider",
                "providers",
                "sourceHealth",
                "state",
                "todayTokens",
            ]
        );
        assert!(!object.contains_key("profileRoot"));
        assert!(!object.contains_key("rawRecord"));
    }

    #[test]
    fn event_name_is_stable() {
        assert_eq!(USAGE_SUMMARY_CHANGED_EVENT, "usage-summary-changed");
    }
}
