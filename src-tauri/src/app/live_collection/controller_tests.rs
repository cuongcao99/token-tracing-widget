use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::super::scheduler::{CollectionReason, LiveCollectionConfig};
use super::{
    update_source_config_and_refresh, CollectionBackend, LiveCollectionController,
    LiveCollectionHandle, SummaryPublisher,
};
use crate::app::runtime::{AppState, RuntimeError};
use crate::collection::{CollectionError, CollectionReport, CollectionStoreError, FixedClock};
use crate::commands::usage_summary::SummaryEventError;
use crate::sources::file_watcher::{SourceObserver, WatchSignal};
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

fn test_controller<B, P>(backend: B, publisher: P, start: Instant) -> LiveCollectionController<B, P>
where
    B: CollectionBackend,
    P: SummaryPublisher,
{
    test_controller_with_state(AppState::unavailable(), backend, publisher, start)
}

fn test_controller_with_state<B, P>(
    state: AppState,
    backend: B,
    publisher: P,
    start: Instant,
) -> LiveCollectionController<B, P>
where
    B: CollectionBackend,
    P: SummaryPublisher,
{
    let (sender, _receiver) = std::sync::mpsc::channel();
    LiveCollectionController::new(
        state,
        backend,
        publisher,
        SourceObserver::new(sender),
        start,
        test_config(),
    )
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
    results: VecDeque<Result<CollectionReport, RuntimeError>>,
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
                Vec::new(),
            )],
        },
        accepted_event_count: 1,
        source_health: Vec::new(),
        has_pending_reads: false,
    }
}

#[test]
fn configuration_changed_refreshes_observed_sources_without_carrying_a_path() {
    let profile = write_profile_with_claude_record(20);
    let database = tempfile::tempdir().unwrap();
    let state = AppState::from_paths(
        profile.path().to_path_buf(),
        &database.path().join("index.sqlite"),
        DiscoveryLimits::new(10, 10_000),
    )
    .unwrap();
    let start = Instant::now();
    let mut live = test_controller(
        ScriptedBackend {
            attempts: 0,
            results: VecDeque::new(),
        },
        RecordingPublisher {
            summaries: Vec::new(),
        },
        start,
    );
    live.state = state;

    assert!(live.on_signal(WatchSignal::ConfigurationChanged, start));
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
    let mut live = test_controller(
        ScriptedBackend {
            attempts: 0,
            results: VecDeque::from([Ok(test_report(20))]),
        },
        RecordingPublisher {
            summaries: Vec::new(),
        },
        start,
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
    let mut live = test_controller(
        ScriptedBackend {
            attempts: 0,
            results: VecDeque::from([Ok(test_report(20))]),
        },
        FailingPublisher,
        start,
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
    let mut live = test_controller(
        ScriptedBackend {
            attempts: 0,
            results: VecDeque::from([
                Err(RuntimeError::Collection(CollectionError::Storage(
                    CollectionStoreError::Write,
                ))),
                Ok(test_report(30)),
            ]),
        },
        RecordingPublisher {
            summaries: Vec::new(),
        },
        start,
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
    let mut live = test_controller_with_state(
        state.clone(),
        FixedClockBackend {
            state,
            clock,
            reports: Vec::new(),
        },
        RecordingPublisher {
            summaries: Vec::new(),
        },
        start,
    );
    live.on_signal(WatchSignal::ConfigurationChanged, start);
    live.on_signal(WatchSignal::Changed(Provider::Claude), start);
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
    let mut live = test_controller_with_state(
        state.clone(),
        FixedClockBackend {
            state,
            clock,
            reports: Vec::new(),
        },
        RecordingPublisher {
            summaries: Vec::new(),
        },
        start,
    );
    live.on_signal(WatchSignal::ConfigurationChanged, start);
    live.scheduler.deactivate();
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
    let mut live = test_controller_with_state(
        state.clone(),
        FixedClockBackend {
            state,
            clock,
            reports: Vec::new(),
        },
        RecordingPublisher {
            summaries: Vec::new(),
        },
        start,
    );
    live.on_signal(WatchSignal::ConfigurationChanged, start);
    live.on_signal(WatchSignal::Changed(Provider::Claude), start);
    live.process_due(start + Duration::from_millis(200));

    assert_eq!(live.publisher.summaries[0].today_tokens, 30);
    assert_eq!(live.backend.reports[0].accepted_event_count, 1);
}

#[test]
fn watch_roots_include_only_existing_native_provider_directories() {
    let profile = write_profile_with_claude_record(20);
    let database = tempfile::tempdir().unwrap();
    let state = AppState::from_paths(
        profile.path().to_path_buf(),
        &database.path().join("index.sqlite"),
        DiscoveryLimits::new(10, 10_000),
    )
    .unwrap();
    let roots = state.watch_roots();

    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].provider(), Provider::Claude);
    assert_eq!(roots[0].path(), profile.path().join(".claude"));
    assert!(AppState::unavailable().watch_roots().is_empty());
}

#[test]
fn shutdown_is_idempotent_and_joins_worker() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let joined = Arc::new(AtomicBool::new(false));
    let joined_by_worker = joined.clone();
    let worker = std::thread::spawn(move || {
        assert_eq!(receiver.recv().unwrap(), WatchSignal::Shutdown);
        joined_by_worker.store(true, Ordering::SeqCst);
    });
    let handle = LiveCollectionHandle::from_parts(sender, worker);

    handle.shutdown();
    handle.shutdown();

    assert!(joined.load(Ordering::SeqCst));
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
    let path = profile.join(r".claude\projects\session.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("Claude fixture should be opened");
    file.write_all(bytes)
        .expect("Claude fixture should be appended");
}
