use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::app::runtime::{AppState, RuntimeError};
use crate::collection::{CollectionReport, WindowsClock};
use crate::commands::usage_summary::{emit_usage_summary, SummaryEventError};
use crate::sources::file_watcher::{SourceObserver, WatchSignal};
use crate::sources::source_config::SourceConfig;
use crate::types::provider::Provider;
use crate::types::usage_summary::UsageSummary;
use crate::usage::session_summary::ACTIVE_SESSION_WINDOW_SECONDS;
use crate::UsageState;

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
    ActivityExpiry,
    Retry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CollectionCommand {
    Observe(Provider),
    Unobserve(Provider),
    Changed(Provider),
    WatchUnavailable(Provider),
    ConfigurationChanged,
    Shutdown,
}

#[derive(Debug)]
pub(crate) struct LiveScheduler {
    config: LiveCollectionConfig,
    notification_deadline: Option<Instant>,
    reconciliation_deadline: Option<Instant>,
    activity_expiry_deadline: Option<Instant>,
    retry_deadline: Option<Instant>,
    retry_attempt: u32,
}

impl LiveScheduler {
    pub(crate) fn new(_start: Instant, config: LiveCollectionConfig) -> Self {
        Self {
            config,
            notification_deadline: None,
            reconciliation_deadline: None,
            activity_expiry_deadline: None,
            retry_deadline: None,
            retry_attempt: 0,
        }
    }

    pub(crate) fn activate(&mut self, now: Instant) {
        if self.reconciliation_deadline.is_none() {
            self.reconciliation_deadline = Some(now + self.config.reconciliation_interval);
        }
    }

    pub(crate) fn deactivate(&mut self) {
        self.notification_deadline = None;
        self.reconciliation_deadline = None;
        self.activity_expiry_deadline = None;
        self.retry_deadline = None;
        self.retry_attempt = 0;
    }

    pub(crate) fn arm_activity_expiry(&mut self, deadline: Instant) {
        self.activity_expiry_deadline = Some(deadline);
    }

    pub(crate) fn clear_activity_expiry(&mut self) {
        self.activity_expiry_deadline = None;
    }

    pub(crate) fn mark_changed(&mut self, now: Instant) {
        let deadline = now + self.config.notification_debounce;
        self.notification_deadline = Some(
            self.notification_deadline
                .map_or(deadline, |existing| existing.min(deadline)),
        );
    }

    pub(crate) fn next_deadline(&self) -> Option<Instant> {
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

pub(crate) trait CollectionBackend: Send {
    fn collect(&mut self) -> Result<CollectionReport, RuntimeError>;
}

pub(crate) trait SummaryPublisher: Send {
    fn publish(&mut self, summary: &UsageSummary) -> Result<(), SummaryEventError>;
}

pub(crate) struct LiveCollectionLoop<B, P> {
    backend: B,
    publisher: P,
    scheduler: LiveScheduler,
    observed_providers: BTreeSet<Provider>,
}

impl<B, P> LiveCollectionLoop<B, P>
where
    B: CollectionBackend,
    P: SummaryPublisher,
{
    pub(crate) fn new(
        backend: B,
        publisher: P,
        start: Instant,
        config: LiveCollectionConfig,
    ) -> Self {
        Self {
            backend,
            publisher,
            scheduler: LiveScheduler::new(start, config),
            observed_providers: BTreeSet::new(),
        }
    }

    pub(crate) fn on_command(&mut self, command: CollectionCommand, now: Instant) -> bool {
        match command {
            CollectionCommand::Observe(provider) => {
                self.observed_providers.insert(provider);
                self.scheduler.activate(now);
                self.scheduler.mark_changed(now);
                true
            }
            CollectionCommand::Unobserve(provider) => {
                self.observed_providers.remove(&provider);
                if self.observed_providers.is_empty() {
                    self.scheduler.deactivate();
                }
                true
            }
            CollectionCommand::Changed(provider)
            | CollectionCommand::WatchUnavailable(provider)
                if self.observed_providers.contains(&provider) =>
            {
                self.scheduler.mark_changed(now);
                true
            }
            CollectionCommand::Changed(_) | CollectionCommand::WatchUnavailable(_) => true,
            CollectionCommand::ConfigurationChanged if !self.observed_providers.is_empty() => {
                self.scheduler.mark_changed(now);
                true
            }
            CollectionCommand::ConfigurationChanged => true,
            CollectionCommand::Shutdown => false,
        }
    }

    pub(crate) fn process_due(&mut self, now: Instant) -> Option<CollectionReason> {
        let reason = self.scheduler.take_due(now)?;
        match self.backend.collect() {
            Ok(report) => {
                let has_pending_reads = report.has_pending_reads;
                if report.summary.state == UsageState::Active {
                    if report.accepted_event_count > 0
                        || self.scheduler.activity_expiry_deadline.is_none()
                    {
                        self.scheduler.arm_activity_expiry(
                            now + Duration::from_secs(ACTIVE_SESSION_WINDOW_SECONDS as u64),
                        );
                    }
                } else {
                    self.scheduler.clear_activity_expiry();
                }
                let publish_error = self.publisher.publish(&report.summary).err();
                self.scheduler.record_success();
                if has_pending_reads && !self.observed_providers.is_empty() {
                    self.scheduler.mark_changed(now);
                }
                if publish_error.is_some() {
                    eprintln!("summary_event:emit");
                }
            }
            Err(_) => self.scheduler.record_failure(now),
        }
        Some(reason)
    }

    pub(crate) fn run(mut self, receiver: Receiver<CollectionCommand>) {
        loop {
            if self.process_due(Instant::now()).is_some() {
                continue;
            }

            let command = match self.scheduler.next_deadline() {
                Some(deadline) => match receiver
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                {
                    Ok(command) => command,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                },
                None => match receiver.recv() {
                    Ok(command) => command,
                    Err(_) => break,
                },
            };
            if !self.on_command(command, Instant::now()) {
                break;
            }
        }
    }
}

struct RuntimeBackend {
    state: AppState,
}

impl RuntimeBackend {
    fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl CollectionBackend for RuntimeBackend {
    fn collect(&mut self) -> Result<CollectionReport, RuntimeError> {
        self.state.collect_once(&WindowsClock::current())
    }
}

struct TauriSummaryPublisher {
    app: tauri::AppHandle,
}

impl TauriSummaryPublisher {
    fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl SummaryPublisher for TauriSummaryPublisher {
    fn publish(&mut self, summary: &UsageSummary) -> Result<(), SummaryEventError> {
        emit_usage_summary(&self.app, summary)
    }
}

struct LiveCollectionController {
    state: AppState,
    observer: SourceObserver,
    collection_sender: Sender<CollectionCommand>,
    collection_worker: Option<JoinHandle<()>>,
    observed_providers: BTreeSet<Provider>,
}

impl LiveCollectionController {
    fn new(
        state: AppState,
        observer: SourceObserver,
        collection_sender: Sender<CollectionCommand>,
        collection_worker: JoinHandle<()>,
    ) -> Self {
        Self {
            state,
            observer,
            collection_sender,
            collection_worker: Some(collection_worker),
            observed_providers: BTreeSet::new(),
        }
    }

    fn send_collection(&self, command: CollectionCommand) {
        let _ = self.collection_sender.send(command);
    }

    fn refresh_observers(&mut self) {
        let roots = self.state.watch_roots();
        let desired_providers = roots
            .iter()
            .map(|root| root.provider())
            .collect::<BTreeSet<_>>();

        for provider in self.observed_providers.clone() {
            if !desired_providers.contains(&provider) {
                self.observer.stop_provider(provider);
                self.observed_providers.remove(&provider);
                self.send_collection(CollectionCommand::Unobserve(provider));
            }
        }

        for root in roots {
            let provider = root.provider();
            self.observer.start_provider(root);
            if self.observed_providers.insert(provider) {
                self.send_collection(CollectionCommand::Observe(provider));
            }
        }

        self.send_collection(CollectionCommand::ConfigurationChanged);
    }

    fn on_signal(&mut self, signal: WatchSignal) -> bool {
        match signal {
            WatchSignal::Changed(provider) => {
                self.send_collection(CollectionCommand::Changed(provider));
                true
            }
            WatchSignal::WatchUnavailable(provider) => {
                self.send_collection(CollectionCommand::WatchUnavailable(provider));
                true
            }
            WatchSignal::ConfigurationChanged => {
                self.refresh_observers();
                true
            }
            WatchSignal::Shutdown => false,
        }
    }

    fn run(mut self, receiver: Receiver<WatchSignal>) {
        self.refresh_observers();
        loop {
            let signal = match receiver.recv() {
                Ok(signal) => signal,
                Err(_) => break,
            };
            if !self.on_signal(signal) {
                break;
            }
        }

        self.observer.shutdown();
        let _ = self.collection_sender.send(CollectionCommand::Shutdown);
        if let Some(worker) = self.collection_worker.take() {
            let _ = worker.join();
        }
    }
}

pub(crate) struct LiveCollectionHandle {
    sender: Mutex<Option<Sender<WatchSignal>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl LiveCollectionHandle {
    #[allow(dead_code)]
    pub(crate) fn request_source_refresh(&self) -> bool {
        let Ok(sender) = self.sender.lock() else {
            return false;
        };
        sender
            .as_ref()
            .is_some_and(|sender| sender.send(WatchSignal::ConfigurationChanged).is_ok())
    }

    pub(crate) fn shutdown(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            if let Some(sender) = sender.take() {
                let _ = sender.send(WatchSignal::Shutdown);
            }
        }
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }

    #[cfg(test)]
    fn from_parts(sender: Sender<WatchSignal>, worker: JoinHandle<()>) -> Self {
        Self {
            sender: Mutex::new(Some(sender)),
            worker: Mutex::new(Some(worker)),
        }
    }
}

#[allow(dead_code)]
pub(crate) fn update_source_config_and_refresh(
    state: &AppState,
    handle: &LiveCollectionHandle,
    config: SourceConfig,
) -> Result<(), RuntimeError> {
    state.update_source_config(config)?;
    let _ = handle.request_source_refresh();
    Ok(())
}

impl Drop for LiveCollectionHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(crate) fn start_live_collection(
    state: AppState,
    app: tauri::AppHandle,
) -> LiveCollectionHandle {
    let (sender, receiver) = mpsc::channel();
    let (collection_sender, collection_receiver) = mpsc::channel();
    let collection_state = state.clone();
    let collection_app = app.clone();
    let collection_worker = thread::Builder::new()
        .name("live-collection".to_owned())
        .spawn(move || {
            LiveCollectionLoop::new(
                RuntimeBackend::new(collection_state),
                TauriSummaryPublisher::new(collection_app),
                Instant::now(),
                LiveCollectionConfig::default(),
            )
            .run(collection_receiver);
        })
        .expect("live collection worker should start");
    let control_sender = sender.clone();
    let control_worker = thread::Builder::new()
        .name("live-collection-controller".to_owned())
        .spawn(move || {
            LiveCollectionController::new(
                state,
                SourceObserver::new(control_sender),
                collection_sender,
                collection_worker,
            )
            .run(receiver);
        })
        .expect("trace control worker should start");

    LiveCollectionHandle {
        sender: Mutex::new(Some(sender)),
        worker: Mutex::new(Some(control_worker)),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        update_source_config_and_refresh, CollectionBackend, CollectionCommand, CollectionReason,
        LiveCollectionConfig, LiveCollectionHandle, LiveCollectionLoop, LiveScheduler,
        SummaryPublisher,
    };
    use crate::app::runtime::{AppState, RuntimeError};
    use crate::collection::{CollectionError, CollectionReport, FixedClock};
    use crate::commands::usage_summary::SummaryEventError;
    use crate::database::connection::StorageError;
    use crate::sources::file_watcher::WatchSignal;
    use crate::sources::session_files::DiscoveryLimits;
    use crate::sources::source_config::SourceConfig;
    use crate::types::provider::Provider;
    use crate::types::provider_usage_summary::ProviderUsageSummary;
    use crate::types::usage_summary::UsageSummary;
    use crate::UsageState;

    fn test_config() -> LiveCollectionConfig {
        LiveCollectionConfig {
            notification_debounce: Duration::from_millis(200),
            reconciliation_interval: Duration::from_secs(30),
            retry_base: Duration::from_secs(1),
            retry_max: Duration::from_secs(30),
        }
    }

    struct RecordingPublisher {
        summaries: Vec<UsageSummary>,
    }

    impl SummaryPublisher for RecordingPublisher {
        fn publish(&mut self, summary: &UsageSummary) -> Result<(), SummaryEventError> {
            self.summaries.push(summary.clone());
            Ok(())
        }
    }

    struct FailingPublisher;

    impl SummaryPublisher for FailingPublisher {
        fn publish(&mut self, _summary: &UsageSummary) -> Result<(), SummaryEventError> {
            Err(SummaryEventError::Emit)
        }
    }

    struct ScriptedBackend {
        attempts: usize,
        results: std::collections::VecDeque<Result<CollectionReport, RuntimeError>>,
    }

    impl CollectionBackend for ScriptedBackend {
        fn collect(&mut self) -> Result<CollectionReport, RuntimeError> {
            self.attempts += 1;
            self.results
                .pop_front()
                .expect("test backend should have a scripted result")
        }
    }

    fn test_report(today_tokens: u64) -> CollectionReport {
        CollectionReport {
            summary: UsageSummary {
                state: UsageState::Active,
                provider: Some("Claude Code".to_owned()),
                current_session_tokens: Some(today_tokens),
                today_tokens,
                last_updated_at: Some("2026-01-01T00:00:00Z".to_owned()),
                source_health: Vec::new(),
                providers: vec![ProviderUsageSummary::new(
                    Provider::Claude,
                    UsageState::Active,
                    Some(today_tokens),
                    today_tokens,
                    Some("2026-01-01T00:00:00Z".to_owned()),
                )],
            },
            accepted_event_count: 1,
            source_health: Vec::new(),
            has_pending_reads: false,
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
        let expiry = start + Duration::from_secs(10);

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

    #[test]
    fn configuration_changed_marks_collection_due_without_carrying_a_path() {
        let start = Instant::now();
        let mut live = LiveCollectionLoop::new(
            ScriptedBackend {
                attempts: 0,
                results: std::collections::VecDeque::new(),
            },
            RecordingPublisher {
                summaries: Vec::new(),
            },
            start,
            test_config(),
        );

        live.on_command(CollectionCommand::Observe(Provider::Claude), start);
        assert!(live.on_command(CollectionCommand::ConfigurationChanged, start));
        assert_eq!(
            live.scheduler.next_deadline(),
            Some(start + test_config().notification_debounce)
        );
    }

    #[test]
    fn source_refresh_sends_only_a_path_free_signal() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            assert_eq!(receiver.recv().unwrap(), WatchSignal::ConfigurationChanged);
            assert_eq!(receiver.recv().unwrap(), WatchSignal::Shutdown);
        });
        let handle = LiveCollectionHandle::from_parts(sender, worker);

        assert!(handle.request_source_refresh());
        handle.shutdown();
    }

    #[test]
    fn successful_source_update_requests_live_refresh() {
        let profile = tempfile::tempdir().unwrap();
        let database = tempfile::tempdir().unwrap();
        let state = AppState::from_paths(
            profile.path().to_path_buf(),
            &database.path().join("index.sqlite"),
            DiscoveryLimits::new(10, 10_000),
        )
        .unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            assert_eq!(receiver.recv().unwrap(), WatchSignal::ConfigurationChanged);
            assert_eq!(receiver.recv().unwrap(), WatchSignal::Shutdown);
        });
        let handle = LiveCollectionHandle::from_parts(sender, worker);
        let config = SourceConfig::try_new(Provider::Claude, false, None).unwrap();

        update_source_config_and_refresh(&state, &handle, config).unwrap();
        handle.shutdown();
    }

    #[test]
    fn successful_attempt_publishes_only_post_commit_summary() {
        let start = Instant::now();
        let mut live = LiveCollectionLoop::new(
            ScriptedBackend {
                attempts: 0,
                results: std::collections::VecDeque::from([Ok(test_report(20))]),
            },
            RecordingPublisher {
                summaries: Vec::new(),
            },
            start,
            test_config(),
        );
        live.scheduler.mark_changed(start);

        assert_eq!(
            live.process_due(start + Duration::from_millis(200)),
            Some(CollectionReason::Notification)
        );
        assert_eq!(live.publisher.summaries[0].today_tokens, 20);
        assert_eq!(live.backend.attempts, 1);
    }

    #[test]
    fn publisher_failure_does_not_turn_committed_collection_into_retry() {
        let start = Instant::now();
        let mut live = LiveCollectionLoop::new(
            ScriptedBackend {
                attempts: 0,
                results: std::collections::VecDeque::from([Ok(test_report(20))]),
            },
            FailingPublisher,
            start,
            test_config(),
        );
        live.scheduler.mark_changed(start);

        assert_eq!(
            live.process_due(start + Duration::from_millis(200)),
            Some(CollectionReason::Notification)
        );
        assert_eq!(live.backend.attempts, 1);
        assert!(live
            .scheduler
            .take_due(start + Duration::from_secs(1))
            .is_none());
    }

    #[test]
    fn failed_storage_attempt_publishes_nothing_and_retries_after_backoff() {
        let start = Instant::now();
        let mut live = LiveCollectionLoop::new(
            ScriptedBackend {
                attempts: 0,
                results: std::collections::VecDeque::from([
                    Err(RuntimeError::Collection(CollectionError::Storage(
                        StorageError::Write,
                    ))),
                    Ok(test_report(30)),
                ]),
            },
            RecordingPublisher {
                summaries: Vec::new(),
            },
            start,
            test_config(),
        );
        live.scheduler.mark_changed(start);

        live.process_due(start + Duration::from_millis(200));
        assert!(live.publisher.summaries.is_empty());
        assert!(live
            .process_due(start + Duration::from_millis(1_199))
            .is_none());

        assert_eq!(
            live.process_due(start + Duration::from_millis(1_200)),
            Some(CollectionReason::Retry)
        );
        assert_eq!(live.publisher.summaries[0].today_tokens, 30);
        assert_eq!(live.backend.attempts, 2);
    }

    #[test]
    fn app_state_notification_collects_appended_record() {
        let profile = write_profile_with_claude_record(20);
        let database = tempfile::tempdir().expect("database directory should be created");
        let state = AppState::from_paths(
            profile.path().to_path_buf(),
            &database.path().join("index.sqlite"),
            DiscoveryLimits::new(10, 10_000),
        )
        .expect("runtime should open");
        let clock = FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01");
        assert_eq!(state.collect_once(&clock).unwrap().summary.today_tokens, 20);

        append_claude_record(profile.path(), 10, "2026-01-01T00:00:01Z");
        let start = Instant::now();
        let mut live = LiveCollectionLoop::new(
            FixedClockBackend {
                state: state.clone(),
                clock,
                reports: Vec::new(),
            },
            RecordingPublisher {
                summaries: Vec::new(),
            },
            start,
            test_config(),
        );
        live.on_command(CollectionCommand::Observe(Provider::Claude), start);
        assert!(live.on_command(CollectionCommand::Changed(Provider::Claude), start));
        live.process_due(start + Duration::from_millis(200));

        assert_eq!(live.publisher.summaries[0].today_tokens, 30);
    }

    #[test]
    fn reconciliation_collects_when_notification_is_missed() {
        let profile = write_profile_with_claude_record(20);
        let database = tempfile::tempdir().expect("database directory should be created");
        let state = AppState::from_paths(
            profile.path().to_path_buf(),
            &database.path().join("index.sqlite"),
            DiscoveryLimits::new(10, 10_000),
        )
        .expect("runtime should open");
        let clock = FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01");
        state
            .collect_once(&clock)
            .expect("initial collection should commit");
        append_claude_record(profile.path(), 10, "2026-01-01T00:00:01Z");

        let start = Instant::now();
        let mut live = LiveCollectionLoop::new(
            FixedClockBackend {
                state,
                clock,
                reports: Vec::new(),
            },
            RecordingPublisher {
                summaries: Vec::new(),
            },
            start,
            test_config(),
        );
        live.scheduler.activate(start);

        assert_eq!(
            live.process_due(start + Duration::from_secs(30)),
            Some(CollectionReason::Reconciliation)
        );
        assert_eq!(live.publisher.summaries[0].today_tokens, 30);
        assert_eq!(live.backend.reports.len(), 1);
    }

    #[test]
    fn partial_final_line_is_completed_by_next_live_collection() {
        let profile = write_profile_with_claude_record(20);
        let second = claude_record("event-2", "2026-01-01T00:00:01Z", 10);
        let split = second.len() / 2;
        append_claude_bytes(profile.path(), &second.as_bytes()[..split]);

        let database = tempfile::tempdir().expect("database directory should be created");
        let state = AppState::from_paths(
            profile.path().to_path_buf(),
            &database.path().join("index.sqlite"),
            DiscoveryLimits::new(10, 10_000),
        )
        .expect("runtime should open");
        let clock = FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01");
        let initial = state
            .collect_once(&clock)
            .expect("initial collection should commit");
        assert_eq!(initial.summary.today_tokens, 20);
        assert_eq!(initial.accepted_event_count, 1);

        append_claude_bytes(profile.path(), &second.as_bytes()[split..]);
        let start = Instant::now();
        let mut live = LiveCollectionLoop::new(
            FixedClockBackend {
                state,
                clock,
                reports: Vec::new(),
            },
            RecordingPublisher {
                summaries: Vec::new(),
            },
            start,
            test_config(),
        );
        live.on_command(CollectionCommand::Observe(Provider::Claude), start);
        live.on_command(CollectionCommand::Changed(Provider::Claude), start);
        live.process_due(start + Duration::from_millis(200));

        assert_eq!(live.publisher.summaries[0].today_tokens, 30);
        assert_eq!(live.backend.reports[0].accepted_event_count, 1);
    }

    #[test]
    fn watch_roots_include_only_existing_native_provider_directories() {
        let profile = write_profile_with_claude_record(20);
        let database = tempfile::tempdir().expect("database directory should be created");
        let state = AppState::from_paths(
            profile.path().to_path_buf(),
            &database.path().join("index.sqlite"),
            DiscoveryLimits::new(10, 10_000),
        )
        .expect("runtime should open");
        let roots = state.watch_roots();

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].provider(), Provider::Claude);
        assert!(AppState::unavailable().watch_roots().is_empty());
    }

    #[test]
    fn shutdown_is_idempotent_and_joins_worker() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let joined = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let joined_by_worker = joined.clone();
        let worker = std::thread::spawn(move || {
            assert_eq!(receiver.recv().unwrap(), WatchSignal::Shutdown);
            joined_by_worker.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        let handle = LiveCollectionHandle::from_parts(sender, worker);

        handle.shutdown();
        handle.shutdown();

        assert!(joined.load(std::sync::atomic::Ordering::SeqCst));
    }

    struct FixedClockBackend {
        state: AppState,
        clock: FixedClock,
        reports: Vec<CollectionReport>,
    }

    impl CollectionBackend for FixedClockBackend {
        fn collect(&mut self) -> Result<CollectionReport, RuntimeError> {
            let result = self.state.collect_once(&self.clock);
            if let Ok(report) = &result {
                self.reports.push(report.clone());
            }
            result
        }
    }

    fn claude_record(event_key: &str, timestamp: &str, total: u64) -> String {
        let input_tokens = total / 2;
        format!(
            "{{\"message\":{{\"id\":\"{event_key}\",\"type\":\"message\",\"usage\":{{\"input_tokens\":{input_tokens},\"output_tokens\":{output_tokens}}}}},\"sessionId\":\"session-a\",\"timestamp\":\"{timestamp}\"}}\n",
            output_tokens = total - input_tokens
        )
    }

    fn write_profile_with_claude_record(total: u64) -> tempfile::TempDir {
        let profile = tempfile::tempdir().expect("profile should be created");
        let root = profile.path().join(r".claude\projects");
        std::fs::create_dir_all(&root).expect("Claude root should be created");
        std::fs::write(
            root.join("session.jsonl"),
            claude_record("event-1", "2026-01-01T00:00:00Z", total),
        )
        .expect("Claude fixture should be written");
        profile
    }

    fn append_claude_record(profile: &std::path::Path, total: u64, timestamp: &str) {
        file_append(
            profile,
            claude_record("event-2", timestamp, total).as_bytes(),
        );
    }

    fn append_claude_bytes(profile: &std::path::Path, bytes: &[u8]) {
        file_append(profile, bytes);
    }

    fn file_append(profile: &std::path::Path, bytes: &[u8]) {
        use std::io::Write;

        let path = profile.join(r".claude\projects\session.jsonl");
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("Claude fixture should be opened");
        file.write_all(bytes)
            .expect("Claude fixture should be appended");
    }
}
