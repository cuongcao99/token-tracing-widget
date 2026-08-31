//! Selecting the provider with the newest valid usage event.

use crate::types::usage_event::UsageEvent;
use crate::usage::session_summary::compute_session_aggregation;
use crate::utils::windows_time::{parse_timestamp_seconds, timestamp_local_day};
use crate::UsageState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveProviderResult {
    pub state: UsageState,
    pub provider: Option<String>,
    pub current_session_tokens: Option<u64>,
    pub last_updated_at: Option<String>,
}

pub fn compute_active_provider(events: &[UsageEvent], now: &str) -> ActiveProviderResult {
    let Some(now_seconds) = parse_timestamp_seconds(now) else {
        return idle_result(None);
    };

    let latest = latest_valid_event(events, now_seconds);

    let Some(latest) = latest else {
        return idle_result(None);
    };
    let provider_events: Vec<_> = events
        .iter()
        .filter(|event| event.provider == latest.provider)
        .cloned()
        .collect();
    let aggregation = compute_session_aggregation(&provider_events, now, None);
    let last_updated_at = Some(latest.observed_at.clone());

    ActiveProviderResult {
        state: aggregation.state,
        provider: Some(latest.provider.display_name().to_owned()),
        current_session_tokens: aggregation.current_session_tokens,
        last_updated_at,
    }
}

/// Computes the current-session total from events observed on the supplied
/// Windows-local calendar day while retaining the all-history active-provider
/// calculation for state and last-update metadata.
pub fn compute_current_session_tokens_for_local_day(
    events: &[UsageEvent],
    now: &str,
    local_day: &str,
) -> Option<u64> {
    if events.is_empty() {
        return None;
    }

    let current_day_events: Vec<_> = events
        .iter()
        .filter(|event| timestamp_local_day(&event.observed_at).as_deref() == Some(local_day))
        .cloned()
        .collect();

    let active_provider = compute_active_provider(&current_day_events, now);
    let Some(provider_name) = active_provider.provider else {
        return Some(0);
    };
    let provider_events: Vec<_> = current_day_events
        .iter()
        .filter(|event| event.provider.display_name() == provider_name)
        .cloned()
        .collect();

    Some(
        compute_session_aggregation(&provider_events, now, Some(local_day))
            .current_session_tokens
            .unwrap_or(0),
    )
}

fn latest_valid_event(events: &[UsageEvent], now_seconds: i64) -> Option<&UsageEvent> {
    events
        .iter()
        .filter_map(|event| {
            let timestamp = parse_timestamp_seconds(&event.observed_at)?;
            (timestamp <= now_seconds).then_some((timestamp, event))
        })
        .max_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.source_position.cmp(&right.1.source_position))
                .then_with(|| left.1.event_id.cmp(&right.1.event_id))
        })
        .map(|(_, event)| event)
}

fn idle_result(last_updated_at: Option<String>) -> ActiveProviderResult {
    ActiveProviderResult {
        state: UsageState::Idle,
        provider: None,
        current_session_tokens: None,
        last_updated_at,
    }
}
