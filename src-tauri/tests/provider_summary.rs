use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::source_health::SourceHealth;
use token_tracing_widget_lib::types::usage_event::UsageEvent;
use token_tracing_widget_lib::usage::provider_summary::compute_provider_summary;
use token_tracing_widget_lib::UsageState;

#[test]
fn computes_session_and_today_totals_per_provider() {
    let events = vec![
        UsageEvent::for_test(
            Provider::Claude,
            "claude-session",
            "2026-01-01T00:00:01Z",
            20,
        ),
        UsageEvent::for_test(
            Provider::Claude,
            "claude-session",
            "2026-01-01T00:00:02Z",
            22,
        ),
        UsageEvent::for_test(Provider::Codex, "codex-session", "2026-01-01T00:00:03Z", 10),
    ];

    let result = compute_provider_summary(
        Provider::Claude,
        &events,
        Some(&SourceHealth::detected(Provider::Claude)),
        "2026-01-01T00:00:04Z",
        "2026-01-01",
    );

    assert_eq!(result.current_session_tokens, Some(42));
    assert_eq!(result.today_tokens, 42);
    assert_eq!(result.state, UsageState::Active);
}

#[test]
fn provider_summary_sums_concurrent_current_day_sessions() {
    let events = vec![
        UsageEvent::for_test(
            Provider::Claude,
            "claude-session-a",
            "2026-01-01T00:01:00Z",
            20,
        ),
        UsageEvent::for_test(
            Provider::Claude,
            "claude-session-b",
            "2026-01-01T00:01:30Z",
            22,
        ),
    ];

    let result = compute_provider_summary(
        Provider::Claude,
        &events,
        Some(&SourceHealth::detected(Provider::Claude)),
        "2026-01-01T00:02:00Z",
        "2026-01-01",
    );

    assert_eq!(result.current_session_tokens, Some(42));
    assert_eq!(result.today_tokens, 42);
    assert_eq!(result.state, UsageState::Active);
}

#[test]
fn resets_current_session_when_latest_event_is_from_a_previous_local_day() {
    let events = vec![UsageEvent::for_test(
        Provider::Claude,
        "claude-session",
        "2026-01-01T00:00:00Z",
        115_265,
    )];

    let result = compute_provider_summary(
        Provider::Claude,
        &events,
        Some(&SourceHealth::detected(Provider::Claude)),
        "2026-01-02T00:00:00Z",
        "2026-01-02",
    );

    assert_eq!(result.current_session_tokens, Some(0));
    assert_eq!(result.today_tokens, 0);
    assert_eq!(result.state, UsageState::Idle);
    assert_eq!(
        result.last_updated_at.as_deref(),
        Some("2026-01-01T00:00:00Z")
    );
}

#[test]
fn preserves_idle_provider_totals_and_marks_missing_source_unavailable() {
    let events = vec![UsageEvent::for_test(
        Provider::Claude,
        "session-a",
        "2026-01-01T00:00:00Z",
        42,
    )];

    let idle = compute_provider_summary(
        Provider::Claude,
        &events,
        Some(&SourceHealth::detected(Provider::Claude)),
        "2026-01-01T00:03:00Z",
        "2026-01-01",
    );
    assert_eq!(idle.state, UsageState::Idle);
    assert_eq!(idle.current_session_tokens, Some(42));

    let unavailable = compute_provider_summary(
        Provider::Codex,
        &events,
        Some(&SourceHealth::new(Provider::Codex, "not_detected")),
        "2026-01-01T00:03:00Z",
        "2026-01-01",
    );
    assert_eq!(unavailable.state, UsageState::Unavailable);
    assert_eq!(unavailable.today_tokens, 0);
}
