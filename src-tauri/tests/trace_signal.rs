use serde_json::{json, Value};
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::trace_signal::{
    ProviderEvent, TraceLifecycle, TraceSignal, TRACE_SIGNAL_SCHEMA_VERSION,
};

fn valid_signal() -> Value {
    json!({
        "schemaVersion": TRACE_SIGNAL_SCHEMA_VERSION,
        "provider": "codex",
        "lifecycle": "start_or_continue",
        "providerEvent": "UserPromptSubmit",
        "observedAt": "2026-09-01T10:00:00.123Z",
        "opaqueSessionId": "session-opaque-001",
        "opaqueTurnId": "turn-opaque-002",
        "sequence": 7
    })
}

#[test]
fn valid_signal_round_trips_only_allowlisted_camel_case_fields() {
    let signal: TraceSignal = serde_json::from_value(valid_signal()).unwrap();

    assert_eq!(signal.schema_version, TRACE_SIGNAL_SCHEMA_VERSION);
    assert_eq!(signal.provider, Provider::Codex);
    assert_eq!(signal.lifecycle, TraceLifecycle::StartOrContinue);
    assert_eq!(signal.provider_event, ProviderEvent::UserPromptSubmit);
    assert_eq!(
        signal.opaque_session_id.as_deref(),
        Some("session-opaque-001")
    );
    assert_eq!(signal.opaque_turn_id.as_deref(), Some("turn-opaque-002"));
    assert_eq!(signal.sequence, Some(7));

    let serialized = serde_json::to_value(signal).unwrap();
    let object = serialized.as_object().unwrap();
    assert_eq!(object.len(), 8);
    assert!(object.contains_key("schemaVersion"));
    assert!(object.contains_key("providerEvent"));
    assert!(object.contains_key("observedAt"));
    assert!(object.contains_key("opaqueSessionId"));
    assert!(object.contains_key("opaqueTurnId"));
    assert!(!serialized.to_string().contains("prompt"));
}

#[test]
fn optional_metadata_is_omitted_when_absent() {
    let signal: TraceSignal = serde_json::from_value(json!({
        "schemaVersion": 1,
        "provider": "claude",
        "lifecycle": "stop",
        "providerEvent": "SessionEnd",
        "observedAt": "2026-09-01T10:00:00Z"
    }))
    .unwrap();

    let serialized = serde_json::to_value(signal).unwrap();
    assert_eq!(serialized.as_object().unwrap().len(), 5);
    assert!(serialized.get("opaqueSessionId").is_none());
    assert!(serialized.get("opaqueTurnId").is_none());
    assert!(serialized.get("sequence").is_none());
}

#[test]
fn supported_provider_events_require_their_normalized_lifecycle() {
    let cases = [
        ("claude", "start_or_continue", "UserPromptSubmit"),
        ("claude", "pause", "Stop"),
        ("claude", "pause", "StopFailure"),
        ("claude", "stop", "SessionEnd"),
        ("codex", "start_or_continue", "SessionStart"),
        ("codex", "start_or_continue", "UserPromptSubmit"),
        ("codex", "pause", "Stop"),
        ("codex", "stop", "SessionEnd"),
    ];

    for (provider, lifecycle, provider_event) in cases {
        let mut value = valid_signal();
        value["provider"] = json!(provider);
        value["lifecycle"] = json!(lifecycle);
        value["providerEvent"] = json!(provider_event);

        serde_json::from_value::<TraceSignal>(value).unwrap();
    }

    for (provider, lifecycle, provider_event) in [
        ("claude", "stop", "UserPromptSubmit"),
        ("claude", "pause", "SessionEnd"),
        ("codex", "pause", "StopFailure"),
        ("codex", "start_or_continue", "Stop"),
    ] {
        let mut value = valid_signal();
        value["provider"] = json!(provider);
        value["lifecycle"] = json!(lifecycle);
        value["providerEvent"] = json!(provider_event);

        assert!(serde_json::from_value::<TraceSignal>(value).is_err());
    }
}

#[test]
fn unknown_fields_cannot_carry_hook_payload_into_the_contract() {
    let mut value = valid_signal();
    value["prompt"] = json!("private prompt must never be retained");
    value["transcriptPath"] = json!("C:/private/transcript.jsonl");
    value["cwd"] = json!("C:/private/repository");

    let error = serde_json::from_value::<TraceSignal>(value)
        .expect_err("arbitrary hook fields must be rejected");
    assert!(!error.to_string().contains("private prompt"));
    assert!(!error.to_string().contains("private/repository"));
}

#[test]
fn invalid_version_provider_lifecycle_and_event_are_rejected() {
    for (field, value) in [
        ("schemaVersion", json!(0)),
        ("schemaVersion", json!(2)),
        ("provider", json!("unknown_provider")),
        ("lifecycle", json!("running")),
        ("providerEvent", json!("UnknownEvent")),
    ] {
        let mut signal = valid_signal();
        signal[field] = value;

        assert!(
            serde_json::from_value::<TraceSignal>(signal).is_err(),
            "{field}"
        );
    }
}

#[test]
fn malformed_or_oversized_opaque_identifiers_are_rejected() {
    for identifier in [
        String::new(),
        "has whitespace".to_owned(),
        "has/slash".to_owned(),
        "has\ncontrol".to_owned(),
        "é".to_owned(),
        "a".repeat(129),
    ] {
        let mut signal = valid_signal();
        signal["opaqueSessionId"] = json!(identifier);

        assert!(serde_json::from_value::<TraceSignal>(signal).is_err());
    }
}

#[test]
fn malformed_or_invalid_calendar_timestamps_are_rejected() {
    for timestamp in [
        "not-a-timestamp",
        "2026-02-30T10:00:00Z",
        "2026-09-01T10:00:00",
        "2026-09-01T10:00:00.1234567890Z",
        "2026-09-01T10:00:00Ztrailing",
    ] {
        let mut signal = valid_signal();
        signal["observedAt"] = json!(timestamp);

        assert!(
            serde_json::from_value::<TraceSignal>(signal).is_err(),
            "{timestamp}"
        );
    }
}
