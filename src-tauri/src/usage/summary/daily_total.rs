use crate::types::usage_event::UsageEvent;
use crate::utils::windows_time::timestamp_local_day;

pub(super) fn compute_today_total(events: &[UsageEvent], local_day: &str) -> u64 {
    events
        .iter()
        .filter(|event| timestamp_local_day(&event.observed_at).as_deref() == Some(local_day))
        .fold(0_u64, |total, event| {
            total.saturating_add(event.total_tokens)
        })
}

#[cfg(test)]
mod tests {
    use super::compute_today_total;
    use crate::types::provider::Provider;
    use crate::types::usage_event::UsageEvent;

    #[test]
    fn local_day_rolls_a_late_utc_event_into_the_next_windows_day() {
        let event = UsageEvent::for_test(Provider::Claude, "session-a", "2026-01-01T23:30:00Z", 20);

        assert_eq!(compute_today_total(&[event], "2026-01-02"), 20);
    }
}
