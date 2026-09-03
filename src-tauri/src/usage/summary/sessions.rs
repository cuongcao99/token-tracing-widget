use std::collections::BTreeMap;

use crate::types::provider::Provider;
use crate::types::session_usage_summary::normalize_session_name;
use crate::types::usage_event::UsageEvent;
use crate::utils::windows_time::{parse_timestamp_seconds, timestamp_local_day};
use crate::UsageState;

pub const ACTIVE_SESSION_WINDOW_SECONDS: i64 = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionAggregate {
    pub(super) session_key: String,
    pub(super) name: Option<String>,
    pub(super) active: bool,
    pub(super) current_day_tokens: u64,
    pub(super) last_updated_at: String,
    last_updated_seconds: i64,
    last_source_position: u64,
    last_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionAggregation {
    pub(super) sessions: Vec<SessionAggregate>,
    pub(super) state: UsageState,
    pub(super) current_session_tokens: Option<u64>,
    pub(super) last_updated_at: Option<String>,
}

pub(super) fn compute_session_aggregation(
    events: &[UsageEvent],
    now: &str,
    local_day: Option<&str>,
) -> SessionAggregation {
    let Some(now_seconds) = parse_timestamp_seconds(now) else {
        return empty_result();
    };

    let mut grouped: BTreeMap<(Provider, String), Vec<&UsageEvent>> = BTreeMap::new();
    for event in events {
        if parse_timestamp_seconds(&event.observed_at)
            .is_some_and(|timestamp| timestamp <= now_seconds)
        {
            grouped
                .entry((event.provider, event.session_key.clone()))
                .or_default()
                .push(event);
        }
    }

    let mut sessions = Vec::with_capacity(grouped.len());
    for ((_, session_key), session_events) in grouped {
        let Some(latest) = latest_event(&session_events) else {
            continue;
        };
        let current_day_events: Vec<_> = session_events
            .iter()
            .copied()
            .filter(|event| {
                local_day.is_none_or(|day| {
                    timestamp_local_day(&event.observed_at).as_deref() == Some(day)
                })
            })
            .collect();
        if local_day.is_some() && current_day_events.is_empty() {
            continue;
        }
        let latest_seconds = parse_timestamp_seconds(&latest.observed_at).unwrap_or(now_seconds);
        let active = now_seconds.saturating_sub(latest_seconds) < ACTIVE_SESSION_WINDOW_SECONDS;
        let current_day_tokens = sum_tokens(&current_day_events);

        sessions.push(SessionAggregate {
            session_key,
            name: latest_session_name(&session_events),
            active,
            current_day_tokens,
            last_updated_at: latest.observed_at.clone(),
            last_updated_seconds: latest_seconds,
            last_source_position: latest.source_position,
            last_event_id: latest.event_id.clone(),
        });
    }

    sessions.sort_by(|left, right| {
        right
            .active
            .cmp(&left.active)
            .then_with(|| right.last_updated_seconds.cmp(&left.last_updated_seconds))
            .then_with(|| left.session_key.cmp(&right.session_key))
    });

    let latest_session = sessions.iter().max_by(|left, right| {
        left.last_updated_seconds
            .cmp(&right.last_updated_seconds)
            .then_with(|| left.last_source_position.cmp(&right.last_source_position))
            .then_with(|| left.last_event_id.cmp(&right.last_event_id))
    });
    let active_session_tokens = sessions
        .iter()
        .filter(|session| session.active)
        .map(|session| session.current_day_tokens)
        .fold(0_u64, u64::saturating_add);
    let has_active_session = sessions.iter().any(|session| session.active);
    let current_session_tokens = if has_active_session {
        Some(active_session_tokens)
    } else {
        latest_session.map(|session| session.current_day_tokens)
    };

    SessionAggregation {
        state: if has_active_session {
            UsageState::Active
        } else {
            UsageState::Idle
        },
        last_updated_at: latest_session.map(|session| session.last_updated_at.clone()),
        current_session_tokens,
        sessions,
    }
}

fn latest_event<'a>(events: &[&'a UsageEvent]) -> Option<&'a UsageEvent> {
    events.iter().copied().max_by(|left, right| {
        timestamp_order(left)
            .cmp(&timestamp_order(right))
            .then_with(|| left.source_position.cmp(&right.source_position))
            .then_with(|| left.event_id.cmp(&right.event_id))
    })
}

fn latest_session_name(events: &[&UsageEvent]) -> Option<String> {
    events
        .iter()
        .copied()
        .filter(|event| normalize_session_name(event.session_name.as_deref()).is_some())
        .max_by(|left, right| {
            timestamp_order(left)
                .cmp(&timestamp_order(right))
                .then_with(|| left.source_position.cmp(&right.source_position))
                .then_with(|| left.event_id.cmp(&right.event_id))
        })
        .and_then(|event| normalize_session_name(event.session_name.as_deref()))
}

fn timestamp_order(event: &UsageEvent) -> i64 {
    parse_timestamp_seconds(&event.observed_at).unwrap_or(i64::MIN)
}

fn sum_tokens(events: &[&UsageEvent]) -> u64 {
    events.iter().fold(0_u64, |total, event| {
        total.saturating_add(event.total_tokens)
    })
}

fn empty_result() -> SessionAggregation {
    SessionAggregation {
        sessions: Vec::new(),
        state: UsageState::Idle,
        current_session_tokens: None,
        last_updated_at: None,
    }
}

#[cfg(test)]
mod tests {
    use super::compute_session_aggregation;
    use crate::types::provider::Provider;
    use crate::types::usage_event::UsageEvent;
    use crate::UsageState;

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

        let result =
            compute_session_aggregation(&events, "2026-01-01T00:01:59Z", Some("2026-01-01"));

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

        let result =
            compute_session_aggregation(&events, "2026-01-01T00:03:00Z", Some("2026-01-01"));

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

        let result =
            compute_session_aggregation(&events, "2026-01-01T00:00:15Z", Some("2026-01-01"));

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

        let result =
            compute_session_aggregation(&events, "2026-01-01T00:00:11Z", Some("2026-01-01"));

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
}
