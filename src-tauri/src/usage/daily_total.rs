//! Calculating the current Windows-local-day total.

use crate::types::usage_event::UsageEvent;
use crate::utils::windows_time::timestamp_local_day;

pub fn compute_today_total(events: &[UsageEvent], local_day: &str) -> u64 {
    events
        .iter()
        .filter(|event| timestamp_local_day(&event.observed_at).as_deref() == Some(local_day))
        .fold(0_u64, |total, event| {
            total.saturating_add(event.total_tokens)
        })
}

#[cfg(test)]
mod tests {
    use super::super::super::utils::windows_time::{
        local_day_from_utc_seconds, parse_timestamp_seconds,
    };

    #[test]
    fn local_day_rolls_a_late_utc_event_into_the_next_windows_day() {
        let timestamp = parse_timestamp_seconds("2026-01-01T23:30:00Z").unwrap();

        assert_eq!(
            local_day_from_utc_seconds(timestamp, 7 * 60 * 60),
            Some("2026-01-02".to_owned())
        );
    }
}
