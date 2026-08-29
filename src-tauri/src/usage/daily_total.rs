//! Calculating the current Windows-local-day total.

use crate::types::usage_event::UsageEvent;
use crate::utils::windows_time::timestamp_local_day;

pub fn compute_today_total(events: &[UsageEvent], local_day: &str) -> u64 {
    events
        .iter()
        .filter(|event| timestamp_local_day(&event.observed_at) == Some(local_day))
        .fold(0_u64, |total, event| {
            total.saturating_add(event.total_tokens)
        })
}
