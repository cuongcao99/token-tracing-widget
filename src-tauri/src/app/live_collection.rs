use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LiveCollectionConfig {
    pub(crate) notification_debounce: Duration,
    pub(crate) reconciliation_interval: Duration,
    pub(crate) retry_base: Duration,
    pub(crate) retry_max: Duration,
}

impl Default for LiveCollectionConfig {
    fn default() -> Self {
        Self {
            notification_debounce: Duration::from_millis(200),
            reconciliation_interval: Duration::from_secs(30),
            retry_base: Duration::from_secs(1),
            retry_max: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CollectionReason {
    Notification,
    Reconciliation,
    Retry,
}

#[derive(Debug)]
pub(crate) struct LiveScheduler {
    config: LiveCollectionConfig,
    notification_deadline: Option<Instant>,
    reconciliation_deadline: Instant,
    retry_deadline: Option<Instant>,
    retry_attempt: u32,
}

impl LiveScheduler {
    pub(crate) fn new(start: Instant, config: LiveCollectionConfig) -> Self {
        Self {
            config,
            notification_deadline: None,
            reconciliation_deadline: start + config.reconciliation_interval,
            retry_deadline: None,
            retry_attempt: 0,
        }
    }

    pub(crate) fn mark_changed(&mut self, now: Instant) {
        let deadline = now + self.config.notification_debounce;
        self.notification_deadline = Some(
            self.notification_deadline
                .map_or(deadline, |existing| existing.min(deadline)),
        );
    }

    pub(crate) fn next_deadline(&self) -> Instant {
        if let Some(retry_deadline) = self.retry_deadline {
            return retry_deadline;
        }

        let mut deadline = self.reconciliation_deadline;
        if let Some(notification_deadline) = self.notification_deadline {
            deadline = deadline.min(notification_deadline);
        }
        deadline
    }

    pub(crate) fn take_due(&mut self, now: Instant) -> Option<CollectionReason> {
        if let Some(retry_deadline) = self.retry_deadline {
            if retry_deadline > now {
                return None;
            }
            self.retry_deadline = None;
            return Some(CollectionReason::Retry);
        }

        if self
            .notification_deadline
            .is_some_and(|deadline| deadline <= now)
        {
            self.notification_deadline = None;
            return Some(CollectionReason::Notification);
        }

        if self.reconciliation_deadline <= now {
            while self.reconciliation_deadline <= now {
                self.reconciliation_deadline += self.config.reconciliation_interval;
            }
            return Some(CollectionReason::Reconciliation);
        }

        None
    }

    pub(crate) fn record_success(&mut self) {
        self.retry_deadline = None;
        self.retry_attempt = 0;
    }

    pub(crate) fn record_failure(&mut self, now: Instant) {
        let multiplier = 1_u32
            .checked_shl(self.retry_attempt.min(31))
            .unwrap_or(u32::MAX);
        let delay = self
            .config
            .retry_base
            .checked_mul(multiplier)
            .unwrap_or(self.config.retry_max)
            .min(self.config.retry_max);
        self.retry_attempt = self.retry_attempt.saturating_add(1);
        self.retry_deadline = Some(now + delay);
    }
}

#[cfg(test)]
mod tests {
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
                failure_at + Duration::from_secs(delay)
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

        assert_eq!(scheduler.next_deadline(), start + Duration::from_secs(30));
    }
}
