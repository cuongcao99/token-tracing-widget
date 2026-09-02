//! Privacy-safe aggregation of opaque provider sessions.

use std::collections::BTreeMap;

use crate::types::provider::Provider;
use crate::types::usage_event::UsageEvent;
use crate::utils::windows_time::{parse_timestamp_seconds, timestamp_local_day};
use crate::UsageState;

pub const ACTIVE_SESSION_WINDOW_SECONDS: i64 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAggregate {
    pub active: bool,
    pub total_tokens: u64,
    pub current_day_tokens: u64,
    pub last_updated_at: String,
    last_updated_seconds: i64,
    last_source_position: u64,
    last_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAggregation {
    pub sessions: Vec<SessionAggregate>,
    pub state: UsageState,
    pub current_session_tokens: Option<u64>,
    pub last_updated_at: Option<String>,
}

pub fn compute_session_aggregation(
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
    for session_events in grouped.into_values() {
        let Some(latest) = latest_event(&session_events) else {
            continue;
        };
        let latest_seconds = parse_timestamp_seconds(&latest.observed_at).unwrap_or(now_seconds);
        let active = now_seconds.saturating_sub(latest_seconds) < ACTIVE_SESSION_WINDOW_SECONDS;
        let total_tokens = sum_tokens(&session_events, |_| true);
        let current_day_tokens = sum_tokens(&session_events, |event| {
            local_day
                .is_none_or(|day| timestamp_local_day(&event.observed_at).as_deref() == Some(day))
        });

        sessions.push(SessionAggregate {
            active,
            total_tokens,
            current_day_tokens,
            last_updated_at: latest.observed_at.clone(),
            last_updated_seconds: latest_seconds,
            last_source_position: latest.source_position,
            last_event_id: latest.event_id.clone(),
        });
    }

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

fn timestamp_order(event: &UsageEvent) -> i64 {
    parse_timestamp_seconds(&event.observed_at).unwrap_or(i64::MIN)
}

fn sum_tokens<F>(events: &[&UsageEvent], include: F) -> u64
where
    F: Fn(&UsageEvent) -> bool,
{
    events
        .iter()
        .filter(|event| include(event))
        .fold(0_u64, |total, event| {
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
