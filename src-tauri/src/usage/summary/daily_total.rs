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
    use crate::utils::windows_time::timestamp_local_day;

    #[test]
    fn includes_event_on_requested_local_day() {
        let observed_at = "2026-01-01T23:30:00Z";
        let event = UsageEvent::for_test(Provider::Claude, "session-a", observed_at, 20);
        let local_day = timestamp_local_day(observed_at).expect("valid test timestamp");

        assert_eq!(compute_today_total(&[event], &local_day), 20);
    }
}
