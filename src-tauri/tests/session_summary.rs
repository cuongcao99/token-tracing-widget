use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::usage_event::UsageEvent;
use token_tracing_widget_lib::usage::session_summary::compute_session_aggregation;
use token_tracing_widget_lib::UsageState;

#[test]
fn aggregates_concurrent_active_sessions_for_active_provider() {
    let events = vec![
        UsageEvent::for_test(
            Provider::Claude,
            "claude-session-a",
            "2026-01-01T00:01:50Z",
            20,
        ),
        UsageEvent::for_test(
            Provider::Claude,
            "claude-session-b",
            "2026-01-01T00:01:55Z",
            22,
        ),
    ];

    let result = compute_session_aggregation(&events, "2026-01-01T00:01:59Z", Some("2026-01-01"));

    assert_eq!(result.state, UsageState::Active);
    assert_eq!(result.current_session_tokens, Some(42));
    assert_eq!(result.sessions.len(), 2);
    assert!(result.sessions.iter().all(|session| session.active));
}

#[test]
fn retains_latest_session_total_when_provider_is_idle() {
    let events = vec![
        UsageEvent::for_test(
            Provider::Claude,
            "claude-session-a",
            "2026-01-01T00:00:00Z",
            20,
        ),
        UsageEvent::for_test(
            Provider::Claude,
            "claude-session-b",
            "2026-01-01T00:00:30Z",
            22,
        ),
    ];

    let result = compute_session_aggregation(&events, "2026-01-01T00:03:00Z", Some("2026-01-01"));

    assert_eq!(result.state, UsageState::Idle);
    assert_eq!(result.current_session_tokens, Some(22));
    assert!(result.sessions.iter().all(|session| !session.active));
}

#[test]
fn treats_a_session_as_idle_after_ten_seconds_without_a_new_event() {
    let events = vec![UsageEvent::for_test(
        Provider::Claude,
        "claude-session-a",
        "2026-01-01T00:00:00Z",
        20,
    )];

    let result = compute_session_aggregation(&events, "2026-01-01T00:00:10Z", Some("2026-01-01"));

    assert_eq!(result.state, UsageState::Idle);
    assert!(result.sessions.iter().all(|session| !session.active));
}
