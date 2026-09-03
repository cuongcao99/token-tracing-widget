use std::time::{Duration, Instant};

use super::{CollectionReason, LiveCollectionConfig, LiveScheduler};

fn test_config() -> LiveCollectionConfig {
    LiveCollectionConfig {
        notification_debounce: Duration::from_millis(200),
        reconciliation_interval: Duration::from_secs(30),
        retry_base: Duration::from_secs(1),
        retry_max: Duration::from_secs(30),
    }
}

#[test]
fn notification_burst_has_one_bounded_debounce_deadline() {
    let start = Instant::now();
    let mut scheduler = LiveScheduler::new(start, test_config());

    scheduler.mark_changed(start);
    scheduler.mark_changed(start + Duration::from_millis(50));

    assert!(scheduler
        .take_due(start + Duration::from_millis(199))
        .is_none());
    assert_eq!(
        scheduler.take_due(start + Duration::from_millis(200)),
        Some(CollectionReason::Notification)
    );
    assert!(scheduler
        .take_due(start + Duration::from_millis(201))
        .is_none());
}

#[test]
fn reconciliation_deadline_is_not_reset_by_notification_collection() {
    let start = Instant::now();
    let mut scheduler = LiveScheduler::new(start, test_config());

    scheduler.activate(start);
    scheduler.mark_changed(start);
    assert_eq!(
        scheduler.take_due(start + Duration::from_millis(200)),
        Some(CollectionReason::Notification)
    );
    scheduler.record_success();

    assert_eq!(
        scheduler.take_due(start + Duration::from_secs(30)),
        Some(CollectionReason::Reconciliation)
    );
}

#[test]
fn retry_backoff_is_exponential_and_capped_at_thirty_seconds() {
    let start = Instant::now();
    let mut scheduler = LiveScheduler::new(start, test_config());
    let delays = [1_u64, 2, 4, 8, 16, 30, 30];
    let mut failure_at = start;

    for delay in delays {
        scheduler.record_failure(failure_at);
        assert_eq!(
            scheduler.next_deadline(),
            Some(failure_at + Duration::from_secs(delay))
        );
        assert_eq!(
            scheduler.take_due(failure_at + Duration::from_secs(delay)),
            Some(CollectionReason::Retry)
        );
        failure_at += Duration::from_secs(delay);
    }
}

#[test]
fn notification_cannot_bypass_pending_retry() {
    let start = Instant::now();
    let mut scheduler = LiveScheduler::new(start, test_config());

    scheduler.record_failure(start);
    scheduler.mark_changed(start + Duration::from_millis(1));

    assert!(scheduler
        .take_due(start + Duration::from_millis(201))
        .is_none());
    assert_eq!(
        scheduler.take_due(start + Duration::from_secs(1)),
        Some(CollectionReason::Retry)
    );
}

#[test]
fn idle_scheduler_waits_until_reconciliation_without_busy_polling() {
    let start = Instant::now();
    let scheduler = LiveScheduler::new(start, test_config());

    assert_eq!(scheduler.next_deadline(), None);
}

#[test]
fn activity_expiry_schedules_a_summary_refresh() {
    let start = Instant::now();
    let mut scheduler = LiveScheduler::new(start, test_config());
    let expiry = start + Duration::from_secs(15);

    scheduler.arm_activity_expiry(expiry);

    assert_eq!(scheduler.next_deadline(), Some(expiry));
    assert!(scheduler
        .take_due(expiry - Duration::from_millis(1))
        .is_none());
    assert_eq!(
        scheduler.take_due(expiry),
        Some(CollectionReason::ActivityExpiry)
    );
}
