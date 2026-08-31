//! Selecting the provider with the newest valid usage event.

use crate::types::usage_event::UsageEvent;
use crate::utils::windows_time::parse_timestamp_seconds;
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

    let latest = events
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
        .map(|(_, event)| event);

    let Some(latest) = latest else {
        return idle_result(None);
    };
    let latest_seconds = parse_timestamp_seconds(&latest.observed_at).unwrap_or(now_seconds);
    let elapsed = now_seconds.saturating_sub(latest_seconds);
    let last_updated_at = Some(latest.observed_at.clone());
    let current_session_tokens = events
        .iter()
        .filter(|event| {
            event.provider == latest.provider
                && event.session_key == latest.session_key
                && parse_timestamp_seconds(&event.observed_at)
                    .is_some_and(|timestamp| timestamp <= now_seconds)
        })
        .fold(0_u64, |total, event| {
            total.saturating_add(event.total_tokens)
        });

    ActiveProviderResult {
        state: if elapsed > 120 {
            UsageState::Idle
        } else {
            UsageState::Active
        },
        provider: Some(latest.provider.display_name().to_owned()),
        current_session_tokens: Some(current_session_tokens),
        last_updated_at,
    }
}

fn idle_result(last_updated_at: Option<String>) -> ActiveProviderResult {
    ActiveProviderResult {
        state: UsageState::Idle,
        provider: None,
        current_session_tokens: None,
        last_updated_at,
    }
}
