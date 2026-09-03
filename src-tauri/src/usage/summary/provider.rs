use crate::types::provider::Provider;
use crate::types::provider_usage_summary::ProviderUsageSummary;
use crate::types::rate_limit::RateLimitSummary;
use crate::types::session_usage_summary::{
    safe_session_id, SessionUsageState, SessionUsageSummary,
};
use crate::types::source_health::SourceHealth;
use crate::types::usage_event::UsageEvent;
use crate::UsageState;

use super::active_provider::{
    compute_active_provider, compute_current_session_tokens_for_local_day,
};
use super::daily_total::compute_today_total;
use super::sessions::compute_session_aggregation;

pub(super) fn compute_provider_summary(
    provider: Provider,
    events: &[UsageEvent],
    health: Option<&SourceHealth>,
    rate_limits: &[RateLimitSummary],
    now: &str,
    local_day: &str,
) -> ProviderUsageSummary {
    let provider_events: Vec<UsageEvent> = events
        .iter()
        .filter(|event| event.provider == provider)
        .cloned()
        .collect();
    let active = compute_active_provider(&provider_events, now);
    let state = if provider_events.is_empty() && !source_is_usable(health) {
        UsageState::Unavailable
    } else if active.state == UsageState::Active {
        UsageState::Active
    } else {
        UsageState::Idle
    };

    let sessions = compute_session_aggregation(&provider_events, now, Some(local_day))
        .sessions
        .into_iter()
        .map(|session| SessionUsageSummary {
            id: safe_session_id(&session.session_key),
            name: session.name,
            state: if session.active {
                SessionUsageState::Active
            } else {
                SessionUsageState::Idle
            },
            today_tokens: session.current_day_tokens,
        })
        .collect();

    let mut summary = ProviderUsageSummary::new(
        provider,
        state,
        compute_current_session_tokens_for_local_day(&provider_events, now, local_day),
        compute_today_total(&provider_events, local_day),
        active.last_updated_at,
        sessions,
    );
    summary.rate_limits = rate_limits.to_vec();
    summary
}

fn source_is_usable(health: Option<&SourceHealth>) -> bool {
    health
        .is_some_and(|health| matches!(health.state.as_str(), "detected" | "limited" | "malformed"))
}

#[cfg(test)]
mod tests {
    use super::compute_provider_summary;
    use crate::types::provider::Provider;
    use crate::types::session_usage_summary::SessionUsageState;
    use crate::types::source_health::SourceHealth;
    use crate::types::usage_event::UsageEvent;
    use crate::UsageState;

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
            &[],
            "2026-01-01T00:00:04Z",
            "2026-01-01",
        );

        assert_eq!(result.current_session_tokens, Some(42));
        assert_eq!(result.today_tokens, 42);
        assert_eq!(result.state, UsageState::Active);
        assert_eq!(result.sessions.len(), 1);
        assert_eq!(result.sessions[0].id, "claude-session");
        assert_eq!(result.sessions[0].today_tokens, 42);
        assert!(matches!(
            result.sessions[0].state,
            SessionUsageState::Active
        ));
    }

    #[test]
    fn provider_summary_sums_concurrent_current_day_sessions() {
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

        let result = compute_provider_summary(
            Provider::Claude,
            &events,
            Some(&SourceHealth::detected(Provider::Claude)),
            &[],
            "2026-01-01T00:01:59Z",
            "2026-01-01",
        );

        assert_eq!(result.current_session_tokens, Some(42));
        assert_eq!(result.today_tokens, 42);
        assert_eq!(result.state, UsageState::Active);
        assert_eq!(
            result
                .sessions
                .iter()
                .map(|session| session.today_tokens)
                .sum::<u64>(),
            result.today_tokens,
        );
        assert_eq!(result.sessions.len(), 2);
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
            &[],
            "2026-01-02T00:00:00Z",
            "2026-01-02",
        );

        assert_eq!(result.current_session_tokens, Some(0));
        assert_eq!(result.today_tokens, 0);
        assert_eq!(result.state, UsageState::Idle);
        assert!(result.sessions.is_empty());
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
            &[],
            "2026-01-01T00:03:00Z",
            "2026-01-01",
        );
        assert_eq!(idle.state, UsageState::Idle);
        assert_eq!(idle.current_session_tokens, Some(42));
        assert_eq!(idle.sessions[0].today_tokens, 42);
        assert!(matches!(idle.sessions[0].state, SessionUsageState::Idle));

        let unavailable = compute_provider_summary(
            Provider::Codex,
            &events,
            Some(&SourceHealth::new(Provider::Codex, "not_detected")),
            &[],
            "2026-01-01T00:03:00Z",
            "2026-01-01",
        );
        assert_eq!(unavailable.state, UsageState::Unavailable);
        assert_eq!(unavailable.today_tokens, 0);
        assert!(unavailable.sessions.is_empty());
    }
}
