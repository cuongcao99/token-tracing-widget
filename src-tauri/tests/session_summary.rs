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
fn treats_a_session_as_idle_after_fifteen_seconds_without_a_new_event() {
    let events = vec![UsageEvent::for_test(
        Provider::Claude,
        "claude-session-a",
        "2026-01-01T00:00:00Z",
        20,
    )];

    let result = compute_session_aggregation(&events, "2026-01-01T00:00:15Z", Some("2026-01-01"));

    assert_eq!(result.state, UsageState::Idle);
    assert!(result.sessions.iter().all(|session| !session.active));
}

#[test]
fn projects_current_day_sessions_active_first_with_stable_order() {
    let mut renamed =
        UsageEvent::for_test(Provider::Claude, "session-b", "2026-01-01T00:00:05Z", 22);
    renamed.session_name = Some("Renamed run".to_owned());
    let events = vec![
        UsageEvent::for_test(Provider::Claude, "old", "2025-12-31T00:00:00Z", 99),
        UsageEvent::for_test(Provider::Claude, "session-a", "2026-01-01T00:00:00Z", 20),
        renamed,
        UsageEvent::for_test(Provider::Claude, "session-c", "2026-01-01T00:00:00Z", 7),
    ];

    let result = compute_session_aggregation(&events, "2026-01-01T00:00:11Z", Some("2026-01-01"));

    assert_eq!(
        result
            .sessions
            .iter()
            .map(|session| session.session_key.as_str())
            .collect::<Vec<_>>(),
        vec!["session-b", "session-a", "session-c"],
    );
    assert_eq!(result.sessions[0].name.as_deref(), Some("Renamed run"));
    assert_eq!(
        result
            .sessions
            .iter()
            .map(|session| session.current_day_tokens)
            .sum::<u64>(),
        49,
    );
}
