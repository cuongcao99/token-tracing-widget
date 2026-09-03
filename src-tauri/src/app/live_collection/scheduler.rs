use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LiveCollectionConfig {
    pub(super) notification_debounce: Duration,
    pub(super) reconciliation_interval: Duration,
    pub(super) retry_base: Duration,
    pub(super) retry_max: Duration,
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
pub(super) enum CollectionReason {
    Notification,
    Reconciliation,
    ActivityExpiry,
    Retry,
}

#[derive(Debug)]
pub(super) struct LiveScheduler {
    config: LiveCollectionConfig,
    notification_deadline: Option<Instant>,
    reconciliation_deadline: Option<Instant>,
    activity_expiry_deadline: Option<Instant>,
    retry_deadline: Option<Instant>,
    retry_attempt: u32,
}

impl LiveScheduler {
    pub(super) fn new(_start: Instant, config: LiveCollectionConfig) -> Self {
        Self {
            config,
            notification_deadline: None,
            reconciliation_deadline: None,
            activity_expiry_deadline: None,
            retry_deadline: None,
            retry_attempt: 0,
        }
    }

    pub(super) fn activate(&mut self, now: Instant) {
        if self.reconciliation_deadline.is_none() {
            self.reconciliation_deadline = Some(now + self.config.reconciliation_interval);
        }
    }

    pub(super) fn deactivate(&mut self) {
        self.notification_deadline = None;
        self.reconciliation_deadline = None;
        self.activity_expiry_deadline = None;
        self.retry_deadline = None;
        self.retry_attempt = 0;
    }

    pub(super) fn arm_activity_expiry(&mut self, deadline: Instant) {
        self.activity_expiry_deadline = Some(deadline);
    }

    pub(super) fn has_activity_expiry(&self) -> bool {
        self.activity_expiry_deadline.is_some()
    }

    pub(super) fn clear_activity_expiry(&mut self) {
        self.activity_expiry_deadline = None;
    }

    pub(super) fn mark_changed(&mut self, now: Instant) {
        let deadline = now + self.config.notification_debounce;
        self.notification_deadline = Some(
            self.notification_deadline
                .map_or(deadline, |existing| existing.min(deadline)),
        );
    }

    pub(super) fn next_deadline(&self) -> Option<Instant> {
        [
            self.retry_deadline,
            self.notification_deadline,
            self.activity_expiry_deadline,
            self.reconciliation_deadline,
        ]
        .into_iter()
        .flatten()
        .min()
    }

    pub(super) fn take_due(&mut self, now: Instant) -> Option<CollectionReason> {
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

        if self
            .activity_expiry_deadline
            .is_some_and(|deadline| deadline <= now)
        {
            self.activity_expiry_deadline = None;
            return Some(CollectionReason::ActivityExpiry);
        }

        if self
            .reconciliation_deadline
            .is_some_and(|deadline| deadline <= now)
        {
            let mut deadline = self
                .reconciliation_deadline
                .expect("reconciliation deadline should be present");
            while deadline <= now {
                deadline += self.config.reconciliation_interval;
            }
            self.reconciliation_deadline = Some(deadline);
            return Some(CollectionReason::Reconciliation);
        }

        None
    }

    pub(super) fn record_success(&mut self) {
        self.retry_deadline = None;
        self.retry_attempt = 0;
    }

    pub(super) fn record_failure(&mut self, now: Instant) {
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
#[path = "scheduler_tests.rs"]
mod tests;
