use token_tracing_widget_lib::commands::widget_settings::WidgetSettingsInput;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::provider_usage_summary::ProviderUsageSummary;
use token_tracing_widget_lib::types::widget_settings::WidgetSettingsSnapshot;
use token_tracing_widget_lib::UsageState;

#[test]
fn provider_summary_serializes_only_normalized_fields() {
    let summary = ProviderUsageSummary::new(
        Provider::Claude,
        UsageState::Idle,
        Some(20),
        40,
        Some("2026-01-01T00:00:00Z".to_owned()),
    );
    let object = serde_json::to_value(summary).unwrap();

    assert_eq!(
        object
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "currentSessionTokens",
            "lastUpdatedAt",
            "provider",
            "state",
            "todayTokens"
        ]
    );
    assert!(!object.to_string().contains("rawRecord"));
}

#[test]
fn widget_settings_serialize_fixed_provider_visibility_and_dark_mode() {
    let snapshot = WidgetSettingsSnapshot::defaults();
    let object = serde_json::to_value(snapshot).unwrap();

    assert_eq!(object["darkMode"], true);
    assert_eq!(object["theme"], "claude");
    assert_eq!(object["visibleProviders"].as_array().unwrap().len(), 2);
}

#[test]
fn widget_input_rejects_unknown_raw_fields() {
    let value = serde_json::json!({
        "visibleProviders": [],
        "darkMode": true,
        "prompt": "private text"
    });

    assert!(serde_json::from_value::<WidgetSettingsInput>(value).is_err());
}
