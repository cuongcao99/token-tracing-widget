use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tempfile::TempDir;
use token_tracing_widget_lib::collection::{
    compute_summary, CollectionCoordinator, CollectionError, CollectionStore, FixedClock,
    ProviderSource, SourceUpdate, SummaryRows, WindowsClock,
};
use token_tracing_widget_lib::database::connection::{IndexStore, StorageError};
use token_tracing_widget_lib::providers::claude::ClaudeReader;
use token_tracing_widget_lib::providers::provider_adapter::{
    ProviderAdapter, ProviderReadError, ProviderReadObservation, ProviderReadResult,
};
use token_tracing_widget_lib::types::file_checkpoint::FileCheckpoint;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::source_health::SourceHealth;
use token_tracing_widget_lib::types::token_observation::{CounterKind, TokenObservation};
use token_tracing_widget_lib::types::usage_event::UsageEvent;
use token_tracing_widget_lib::usage::cumulative_delta::convert_observations;
use token_tracing_widget_lib::usage::observation_validation::{
    validate_observation, ObservationValidationError,
};

fn codex_observation(timestamp: &str, total: u64) -> TokenObservation {
    let input_tokens = total / 2;
    TokenObservation {
        provider: Provider::Codex,
        source_session_key: None,
        source_event_key: None,
        observed_at: timestamp.to_owned(),
        counter_kind: CounterKind::Cumulative,
        input_tokens: Some(input_tokens),
        cached_input_tokens: Some(total),
        output_tokens: Some(total - input_tokens),
        total_tokens: total,
    }
}

#[test]
fn summary_contains_independent_provider_totals_for_the_overlay() {
    let summary = compute_summary(
        &SummaryRows {
            events: vec![
                UsageEvent::for_test(
                    Provider::Claude,
                    "claude-session",
                    "2026-01-01T00:00:01Z",
                    12,
                ),
                UsageEvent::for_test(
                    Provider::Claude,
                    "claude-session",
                    "2026-01-01T00:00:02Z",
                    3,
                ),
                UsageEvent::for_test(Provider::Codex, "codex-session", "2026-01-01T00:00:03Z", 8),
            ],
        },
        &[
            SourceHealth::detected(Provider::Claude),
            SourceHealth::detected(Provider::Codex),
        ],
        &[Provider::Claude, Provider::Codex],
        &FixedClock::new("2026-01-01T00:00:04Z", "2026-01-01"),
    );

    assert_eq!(summary.today_tokens, 23);
    assert_eq!(summary.providers.len(), 2);

    let claude = &summary.providers[0];
    assert_eq!(claude.provider, Provider::Claude);
    assert_eq!(claude.current_session_tokens, Some(15));
    assert_eq!(claude.today_tokens, 15);

    let codex = &summary.providers[1];
    assert_eq!(codex.provider, Provider::Codex);
    assert_eq!(codex.current_session_tokens, Some(8));
    assert_eq!(codex.today_tokens, 8);
}

#[test]
fn aggregate_current_session_resets_after_the_windows_local_day_changes() {
    let summary = compute_summary(
        &SummaryRows {
            events: vec![UsageEvent::for_test(
                Provider::Claude,
                "claude-session",
                "2026-01-01T00:00:00Z",
                115_265,
            )],
        },
        &[SourceHealth::detected(Provider::Claude)],
        &[Provider::Claude],
        &FixedClock::new("2026-01-02T00:00:00Z", "2026-01-02"),
    );

    assert_eq!(summary.current_session_tokens, Some(0));
    assert_eq!(summary.today_tokens, 0);
    assert_eq!(summary.provider.as_deref(), Some("Claude Code"));
    assert_eq!(summary.state, token_tracing_widget_lib::UsageState::Idle);
    assert_eq!(
        summary.last_updated_at.as_deref(),
        Some("2026-01-01T00:00:00Z")
    );
}

#[test]
fn cumulative_snapshots_become_deltas_and_reset_starts_new_segment() {
    let observations = vec![
        ProviderReadObservation::new(codex_observation("2026-01-01T00:00:00Z", 10), 0),
        ProviderReadObservation::new(codex_observation("2026-01-01T00:00:01Z", 20), 100),
        ProviderReadObservation::new(codex_observation("2026-01-01T00:00:02Z", 5), 200),
    ];
    let checkpoint = FileCheckpoint::new("file-a", Provider::Codex);
    let batch = convert_observations("file-a", &checkpoint, observations).unwrap();

    assert_eq!(
        batch
            .events
            .iter()
            .map(|event| event.total_tokens)
            .collect::<Vec<_>>(),
        vec![10, 10, 5]
    );
    assert_eq!(batch.events[0].monotonic_segment, 0);
    assert_eq!(batch.events[2].monotonic_segment, 1);
}

#[test]
fn duplicate_stable_event_key_is_accepted_once() {
    let observation = TokenObservation {
        provider: Provider::Claude,
        source_session_key: Some("session-a".to_owned()),
        source_event_key: Some("event-1".to_owned()),
        observed_at: "2026-01-01T00:00:00Z".to_owned(),
        counter_kind: CounterKind::Incremental,
        input_tokens: Some(10),
        cached_input_tokens: Some(4),
        output_tokens: Some(10),
        total_tokens: 20,
    };
    let observations = vec![ProviderReadObservation::new(observation, 0)];
    let checkpoint = FileCheckpoint::new("file-a", Provider::Claude);
    let first = convert_observations("file-a", &checkpoint, observations.clone()).unwrap();
    let second = convert_observations("file-a", &first.next_checkpoint, observations).unwrap();

    assert_eq!(first.events.len(), 1);
    assert!(second.events.is_empty());
}

#[test]
fn duplicate_stable_event_key_is_rejected_even_at_a_new_source_position() {
    let observation = TokenObservation {
        provider: Provider::Claude,
        source_session_key: Some("session-a".to_owned()),
        source_event_key: Some("event-1".to_owned()),
        observed_at: "2026-01-01T00:00:00Z".to_owned(),
        counter_kind: CounterKind::Incremental,
        input_tokens: Some(10),
        cached_input_tokens: Some(4),
        output_tokens: Some(10),
        total_tokens: 20,
    };
    let first = convert_observations(
        "file-a",
        &FileCheckpoint::new("file-a", Provider::Claude),
        vec![ProviderReadObservation::new(observation.clone(), 0)],
    )
    .unwrap();
    let second = convert_observations(
        "file-a",
        &first.next_checkpoint,
        vec![ProviderReadObservation::new(observation, 100)],
    )
    .unwrap();

    assert!(second.events.is_empty());
}

fn incremental_observation(
    event_key: Option<&str>,
    timestamp: &str,
    input_tokens: u64,
    output_tokens: u64,
) -> TokenObservation {
    TokenObservation {
        provider: Provider::Claude,
        source_session_key: Some("session-a".to_owned()),
        source_event_key: event_key.map(str::to_owned),
        observed_at: timestamp.to_owned(),
        counter_kind: CounterKind::Incremental,
        input_tokens: Some(input_tokens),
        cached_input_tokens: None,
        output_tokens: Some(output_tokens),
        total_tokens: input_tokens + output_tokens,
    }
}

#[test]
fn observations_are_ordered_by_timestamp_then_source_position() {
    let observations = vec![
        ProviderReadObservation::new(
            incremental_observation(Some("late"), "2026-01-01T00:00:01Z", 2, 3),
            20,
        ),
        ProviderReadObservation::new(
            incremental_observation(Some("early"), "2026-01-01T00:00:00Z", 4, 6),
            10,
        ),
        ProviderReadObservation::new(
            incremental_observation(Some("same-time"), "2026-01-01T00:00:01Z", 1, 1),
            15,
        ),
    ];
    let batch = convert_observations(
        "file-a",
        &FileCheckpoint::new("file-a", Provider::Claude),
        observations,
    )
    .unwrap();

    let keys: Vec<_> = batch
        .events
        .iter()
        .map(|event| event.observed_at.as_str())
        .collect();
    assert_eq!(
        keys,
        vec![
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:01Z",
            "2026-01-01T00:00:01Z"
        ]
    );
    assert_eq!(batch.events[1].source_position, 15);
    assert_eq!(batch.events[2].source_position, 20);
}

#[test]
fn missing_source_session_key_falls_back_to_opaque_file_identity() {
    let batch = convert_observations(
        "opaque-file-id",
        &FileCheckpoint::new("opaque-file-id", Provider::Codex),
        vec![ProviderReadObservation::new(
            codex_observation("2026-01-01T00:00:00Z", 10),
            0,
        )],
    )
    .unwrap();

    assert_eq!(batch.events[0].session_key, "opaque-file-id");
    assert!(!batch.events[0].event_id.contains("opaque-file-id"));
}

#[test]
fn validation_rejects_inconsistent_total_and_checked_add_overflow() {
    let inconsistent = incremental_observation(Some("bad-total"), "2026-01-01T00:00:00Z", 10, 5);
    assert_eq!(
        validate_observation(&TokenObservation {
            total_tokens: 20,
            ..inconsistent
        }),
        Err(ObservationValidationError::TotalMismatch)
    );

    let overflowing = TokenObservation {
        input_tokens: Some(u64::MAX),
        output_tokens: Some(1),
        total_tokens: 0,
        ..incremental_observation(Some("overflow"), "2026-01-01T00:00:00Z", 0, 0)
    };
    assert_eq!(
        validate_observation(&overflowing),
        Err(ObservationValidationError::CounterOverflow)
    );
}

#[test]
fn cached_input_changes_do_not_inflate_total_delta() {
    let first = TokenObservation {
        cached_input_tokens: Some(2),
        ..codex_observation("2026-01-01T00:00:00Z", 10)
    };
    let second = TokenObservation {
        cached_input_tokens: Some(7),
        ..codex_observation("2026-01-01T00:00:01Z", 10)
    };
    let batch = convert_observations(
        "file-a",
        &FileCheckpoint::new("file-a", Provider::Codex),
        vec![
            ProviderReadObservation::new(first, 0),
            ProviderReadObservation::new(second, 100),
        ],
    )
    .unwrap();

    assert_eq!(batch.events[0].total_tokens, 10);
    assert_eq!(batch.events[1].total_tokens, 0);
    assert_eq!(batch.events[1].cached_input_tokens, Some(5));
    assert_eq!(
        batch
            .events
            .iter()
            .map(|event| event.total_tokens)
            .sum::<u64>(),
        10
    );
}

#[derive(Default)]
struct InMemoryStore {
    events: Vec<UsageEvent>,
    checkpoints: HashMap<String, FileCheckpoint>,
    source_updates: Arc<Mutex<Vec<SourceUpdate>>>,
    failure: Option<StorageError>,
}

impl CollectionStore for InMemoryStore {
    fn load_checkpoint(&self, identity: &str) -> Result<Option<FileCheckpoint>, StorageError> {
        Ok(self.checkpoints.get(identity).cloned())
    }

    fn apply_batch(
        &mut self,
        batch: &token_tracing_widget_lib::collection::CollectionBatch,
    ) -> Result<(), StorageError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.source_updates
            .lock()
            .unwrap()
            .extend(batch.source_updates.iter().cloned());
        let mut known_event_ids: HashSet<_> = self
            .events
            .iter()
            .map(|event| event.event_id.clone())
            .collect();
        self.events.extend(
            batch
                .events
                .iter()
                .filter(|event| known_event_ids.insert(event.event_id.clone()))
                .cloned(),
        );
        for checkpoint in &batch.checkpoints {
            self.checkpoints
                .insert(checkpoint.file_identity.clone(), checkpoint.clone());
        }
        Ok(())
    }

    fn query_events_for_summary(
        &self,
        _day_start: &str,
        _now: &str,
    ) -> Result<SummaryRows, StorageError> {
        Ok(SummaryRows {
            events: self.events.clone(),
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct AlwaysFailReader;

impl ProviderAdapter for AlwaysFailReader {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn read_observations(
        &self,
        _file: &Path,
        _start_offset: u64,
    ) -> Result<ProviderReadResult, ProviderReadError> {
        Err(ProviderReadError::Io)
    }
}

fn test_profile() -> (TempDir, PathBuf) {
    let profile = tempfile::tempdir().unwrap();
    let root = profile.path().join(r".claude\projects");
    let codex_root = profile.path().join(r".codex\sessions");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&codex_root).unwrap();
    fs::write(
        root.join("session.jsonl"),
        br#"{"message":{"id":"event-1","type":"message","usage":{"input_tokens":10,"output_tokens":10}},"sessionId":"session-a","timestamp":"2026-01-01T00:00:00Z"}
"#,
    )
    .unwrap();
    fs::write(codex_root.join("session.jsonl"), b"codex metadata only\n").unwrap();
    (profile, root)
}

fn test_sources_with_one_broken_codex() -> (
    CollectionCoordinator<InMemoryStore>,
    Vec<ProviderSource<'static>>,
) {
    let (profile, _root) = test_profile();
    let profile = Box::leak(Box::new(profile));
    let claude_reader: &'static ClaudeReader = Box::leak(Box::new(ClaudeReader::default()));
    let codex_reader: &'static AlwaysFailReader = Box::leak(Box::new(AlwaysFailReader));
    let [claude_discovery, codex_discovery] =
        token_tracing_widget_lib::sources::session_files::discover_native_sources(
            profile.path(),
            token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
        );

    let sources = vec![
        ProviderSource::new(true, claude_discovery, claude_reader),
        ProviderSource::new(true, codex_discovery, codex_reader),
    ];
    (
        CollectionCoordinator::new(InMemoryStore::default()),
        sources,
    )
}

fn test_sources_with_failing_store() -> (
    CollectionCoordinator<InMemoryStore>,
    Vec<ProviderSource<'static>>,
) {
    let (profile, _root) = test_profile();
    let profile = Box::leak(Box::new(profile));
    let claude_reader: &'static ClaudeReader = Box::leak(Box::new(ClaudeReader::default()));
    let codex_reader: &'static AlwaysFailReader = Box::leak(Box::new(AlwaysFailReader));
    let [claude_discovery, codex_discovery] =
        token_tracing_widget_lib::sources::session_files::discover_native_sources(
            profile.path(),
            token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
        );
    let sources = vec![
        ProviderSource::new(true, claude_discovery, claude_reader),
        ProviderSource::new(true, codex_discovery, codex_reader),
    ];
    let store = InMemoryStore {
        failure: Some(StorageError::Write),
        ..InMemoryStore::default()
    };
    (CollectionCoordinator::new(store), sources)
}

#[test]
fn one_provider_failure_does_not_block_the_other_provider() {
    let (mut coordinator, sources) = test_sources_with_one_broken_codex();
    let report = coordinator
        .collect(
            &sources,
            &FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"),
        )
        .unwrap();

    assert_eq!(report.summary.today_tokens, 20);
    assert_eq!(report.summary.source_health[0].state, "detected");
    assert_eq!(report.summary.source_health[1].state, "unavailable");
}

#[test]
fn summary_is_not_recomputed_when_sqlite_commit_fails() {
    let (mut coordinator, sources) = test_sources_with_failing_store();
    let result = coordinator.collect(
        &sources,
        &FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"),
    );

    assert!(matches!(
        result,
        Err(CollectionError::Storage(StorageError::Write))
    ));
    assert_eq!(
        coordinator.last_summary().state,
        token_tracing_widget_lib::UsageState::Stale
    );
}

#[test]
fn active_provider_expires_after_two_minutes_but_last_update_remains() {
    let events = vec![UsageEvent::for_test(
        Provider::Claude,
        "session-a",
        "2026-01-01T10:00:00Z",
        20,
    )];
    let source_health = vec![SourceHealth::detected(Provider::Claude)];
    let summary = compute_summary(
        &SummaryRows { events },
        &source_health,
        &[Provider::Claude],
        &FixedClock::new("2026-01-01T10:02:01Z", "2026-01-01"),
    );

    assert_eq!(summary.state, token_tracing_widget_lib::UsageState::Idle);
    assert_eq!(summary.provider.as_deref(), Some("Claude Code"));
    assert_eq!(summary.current_session_tokens, Some(20));
    assert_eq!(
        summary.last_updated_at.as_deref(),
        Some("2026-01-01T10:00:00Z")
    );
    assert_eq!(summary.today_tokens, 20);
}

#[test]
fn today_total_combines_enabled_providers_without_double_counting_cumulative_snapshots() {
    let events = vec![
        UsageEvent::for_test(Provider::Claude, "session-a", "2026-01-01T10:00:00Z", 20),
        UsageEvent::for_test(Provider::Codex, "file-b", "2026-01-01T10:00:01Z", 20),
    ];
    let source_health = vec![
        SourceHealth::detected(Provider::Claude),
        SourceHealth::detected(Provider::Codex),
    ];
    let summary = compute_summary(
        &SummaryRows { events },
        &source_health,
        &[Provider::Claude, Provider::Codex],
        &FixedClock::new("2026-01-01T10:00:30Z", "2026-01-01"),
    );

    assert_eq!(summary.today_tokens, 40);
}

#[test]
fn windows_clock_provides_parseable_now_and_local_day() {
    let clock = WindowsClock::current();

    assert!(
        token_tracing_widget_lib::utils::windows_time::parse_timestamp_seconds(clock.now())
            .is_some()
    );
    assert_eq!(clock.local_day().len(), 10);
    assert_eq!(&clock.local_day()[4..5], "-");
    assert_eq!(&clock.local_day()[7..8], "-");
}

#[test]
fn future_events_do_not_inflate_the_current_session_total() {
    let summary = compute_summary(
        &SummaryRows {
            events: vec![
                UsageEvent::for_test(Provider::Claude, "session-a", "2026-01-01T10:00:00Z", 20),
                UsageEvent::for_test(Provider::Claude, "session-a", "2026-01-01T10:01:00Z", 100),
            ],
        },
        &[SourceHealth::detected(Provider::Claude)],
        &[Provider::Claude],
        &FixedClock::new("2026-01-01T10:00:30Z", "2026-01-01"),
    );

    assert_eq!(summary.current_session_tokens, Some(20));
}

#[test]
fn disabled_provider_events_do_not_enter_summary_totals() {
    let rows = SummaryRows {
        events: vec![
            UsageEvent::for_test(
                Provider::Claude,
                "claude-session",
                "2026-01-01T10:00:00Z",
                20,
            ),
            UsageEvent::for_test(Provider::Codex, "codex-session", "2026-01-01T10:00:01Z", 30),
        ],
    };
    let health = vec![
        SourceHealth::detected(Provider::Claude),
        SourceHealth::new(Provider::Codex, "disabled"),
    ];

    let summary = compute_summary(
        &rows,
        &health,
        &[Provider::Claude],
        &FixedClock::new("2026-01-01T10:00:30Z", "2026-01-01"),
    );

    assert_eq!(summary.today_tokens, 20);
    assert_eq!(summary.provider.as_deref(), Some("Claude Code"));
}

#[test]
fn source_update_preserves_explicit_configured_root_label() {
    let profile = tempfile::tempdir().unwrap();
    let root = profile.path().join("custom-source");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("session.jsonl"),
        claude_record("event-1", "2026-01-01T10:00:00Z", 20),
    )
    .unwrap();
    let config = token_tracing_widget_lib::sources::source_config::SourceConfig::try_new(
        Provider::Claude,
        true,
        Some(root.clone()),
    )
    .unwrap();
    let label = root.to_string_lossy().into_owned();
    let discovery = token_tracing_widget_lib::sources::session_files::discover_configured_source(
        profile.path(),
        &config,
        token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
    );
    let reader: &'static ClaudeReader = Box::leak(Box::new(ClaudeReader::default()));
    let store = InMemoryStore::default();
    let updates = Arc::clone(&store.source_updates);
    let mut coordinator = CollectionCoordinator::new(store);
    let source =
        ProviderSource::with_configured_root(true, label.clone(), false, discovery, reader);

    coordinator
        .collect(
            &[source],
            &FixedClock::new("2026-01-01T10:00:30Z", "2026-01-01"),
        )
        .unwrap();

    assert_eq!(updates.lock().unwrap()[0].configured_root, label);
}

fn claude_record(event_key: &str, timestamp: &str, total: u64) -> String {
    let input_tokens = total / 2;
    format!(
        "{{\"message\":{{\"id\":\"{event_key}\",\"type\":\"message\",\"usage\":{{\"input_tokens\":{input_tokens},\"output_tokens\":{output_tokens}}}}},\"sessionId\":\"session-a\",\"timestamp\":\"{timestamp}\"}}\n",
        output_tokens = total - input_tokens
    )
}

fn codex_record(timestamp: &str, total: u64) -> String {
    let input_tokens = total / 2;
    format!(
        "{{\"payload\":{{\"info\":{{\"total_token_usage\":{{\"input_tokens\":{input_tokens},\"output_tokens\":{output_tokens},\"total_tokens\":{total}}}}},\"type\":\"token_count\"}},\"timestamp\":\"{timestamp}\"}}\n",
        output_tokens = total - input_tokens
    )
}

#[test]
fn restart_append_and_rotation_preserve_totals_and_dedupe_events() {
    let profile = tempfile::tempdir().unwrap();
    let profile = Box::leak(Box::new(profile));
    let claude_root = profile.path().join(r".claude\projects");
    fs::create_dir_all(&claude_root).unwrap();
    let session_file = claude_root.join("session.jsonl");
    fs::write(
        &session_file,
        claude_record("event-1", "2026-01-01T00:00:00Z", 20),
    )
    .unwrap();
    let reader: &'static ClaudeReader = Box::leak(Box::new(ClaudeReader::default()));
    let mut coordinator = CollectionCoordinator::new(InMemoryStore::default());
    let clock = FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01");

    let sources = {
        let [discovery, _] =
            token_tracing_widget_lib::sources::session_files::discover_native_sources(
                profile.path(),
                token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
            );
        vec![ProviderSource::new(true, discovery, reader)]
    };
    assert_eq!(
        coordinator
            .collect(&sources, &clock)
            .unwrap()
            .summary
            .today_tokens,
        20
    );

    assert_eq!(
        coordinator
            .collect(&sources, &clock)
            .unwrap()
            .summary
            .today_tokens,
        20
    );

    fs::OpenOptions::new()
        .append(true)
        .open(&session_file)
        .unwrap()
        .write_all(claude_record("event-2", "2026-01-01T00:00:01Z", 10).as_bytes())
        .unwrap();
    let sources = {
        let [discovery, _] =
            token_tracing_widget_lib::sources::session_files::discover_native_sources(
                profile.path(),
                token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
            );
        vec![ProviderSource::new(true, discovery, reader)]
    };
    assert_eq!(
        coordinator
            .collect(&sources, &clock)
            .unwrap()
            .summary
            .today_tokens,
        30
    );

    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(
        &session_file,
        format!(
            "{}{}",
            claude_record("event-1", "2026-01-01T00:00:00Z", 20),
            claude_record("event-3", "2026-01-01T00:00:02Z", 5)
        ),
    )
    .unwrap();
    let sources = {
        let [discovery, _] =
            token_tracing_widget_lib::sources::session_files::discover_native_sources(
                profile.path(),
                token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
            );
        vec![ProviderSource::new(true, discovery, reader)]
    };
    let report = coordinator.collect(&sources, &clock).unwrap();
    assert_eq!(report.summary.today_tokens, 35);
}

#[test]
fn partial_write_completion_is_collected_on_the_next_scan() {
    let profile = tempfile::tempdir().unwrap();
    let profile = Box::leak(Box::new(profile));
    let root = profile.path().join(r".claude\projects");
    fs::create_dir_all(&root).unwrap();
    let session_file = root.join("session.jsonl");
    let first = claude_record("event-1", "2026-01-01T00:00:00Z", 20);
    let second = claude_record("event-2", "2026-01-01T00:00:01Z", 10);
    let split_at = second.len() / 2;
    fs::write(&session_file, format!("{first}{}", &second[..split_at])).unwrap();
    let reader: &'static ClaudeReader = Box::leak(Box::new(ClaudeReader::default()));
    let mut coordinator = CollectionCoordinator::new(InMemoryStore::default());
    let clock = FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01");

    let sources = {
        let [discovery, _] =
            token_tracing_widget_lib::sources::session_files::discover_native_sources(
                profile.path(),
                token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
            );
        vec![ProviderSource::new(true, discovery, reader)]
    };
    assert_eq!(
        coordinator
            .collect(&sources, &clock)
            .unwrap()
            .summary
            .today_tokens,
        20
    );

    fs::OpenOptions::new()
        .append(true)
        .open(&session_file)
        .unwrap()
        .write_all(second[split_at..].as_bytes())
        .unwrap();
    let sources = {
        let [discovery, _] =
            token_tracing_widget_lib::sources::session_files::discover_native_sources(
                profile.path(),
                token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
            );
        vec![ProviderSource::new(true, discovery, reader)]
    };
    assert_eq!(
        coordinator
            .collect(&sources, &clock)
            .unwrap()
            .summary
            .today_tokens,
        30
    );
}

#[test]
fn concurrent_claude_and_codex_sources_are_collected_independently() {
    let profile = tempfile::tempdir().unwrap();
    let profile = Box::leak(Box::new(profile));
    let claude_root = profile.path().join(r".claude\projects");
    let codex_root = profile.path().join(r".codex\sessions");
    fs::create_dir_all(&claude_root).unwrap();
    fs::create_dir_all(&codex_root).unwrap();
    fs::write(
        claude_root.join("claude.jsonl"),
        claude_record("event-1", "2026-01-01T00:00:00Z", 20),
    )
    .unwrap();
    fs::write(
        codex_root.join("codex.jsonl"),
        codex_record("2026-01-01T00:00:01Z", 10),
    )
    .unwrap();
    let claude_reader: &'static ClaudeReader = Box::leak(Box::new(ClaudeReader::default()));
    let codex_reader: &'static token_tracing_widget_lib::providers::codex::CodexReader = Box::leak(
        Box::new(token_tracing_widget_lib::providers::codex::CodexReader::default()),
    );
    let [claude_discovery, codex_discovery] =
        token_tracing_widget_lib::sources::session_files::discover_native_sources(
            profile.path(),
            token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
        );
    let sources = vec![
        ProviderSource::new(true, claude_discovery, claude_reader),
        ProviderSource::new(true, codex_discovery, codex_reader),
    ];
    let mut coordinator = CollectionCoordinator::new(InMemoryStore::default());

    let report = coordinator
        .collect(
            &sources,
            &FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"),
        )
        .unwrap();
    assert_eq!(report.summary.today_tokens, 30);
    assert_eq!(report.source_health.len(), 2);
    assert!(report
        .source_health
        .iter()
        .all(|health| health.state == "detected"));
}

#[test]
fn sqlite_restart_preserves_collected_total_without_rescanning_committed_records() {
    let profile = tempfile::tempdir().unwrap();
    let profile = Box::leak(Box::new(profile));
    let root = profile.path().join(r".claude\projects");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("session.jsonl"),
        claude_record("event-1", "2026-01-01T00:00:00Z", 20),
    )
    .unwrap();
    let database_directory = tempfile::tempdir().unwrap();
    let database_path = database_directory.path().join("index.sqlite");
    let reader: &'static ClaudeReader = Box::leak(Box::new(ClaudeReader::default()));
    let clock = FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01");

    let [discovery, _] = token_tracing_widget_lib::sources::session_files::discover_native_sources(
        profile.path(),
        token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
    );
    let sources = vec![ProviderSource::new(true, discovery, reader)];
    let mut first_coordinator =
        CollectionCoordinator::new(IndexStore::open(&database_path).unwrap());
    assert_eq!(
        first_coordinator
            .collect(&sources, &clock)
            .unwrap()
            .summary
            .today_tokens,
        20
    );
    drop(first_coordinator);

    let [discovery, _] = token_tracing_widget_lib::sources::session_files::discover_native_sources(
        profile.path(),
        token_tracing_widget_lib::sources::session_files::DiscoveryLimits::new(10, 10_000),
    );
    let sources = vec![ProviderSource::new(true, discovery, reader)];
    let mut restarted_coordinator =
        CollectionCoordinator::new(IndexStore::open(&database_path).unwrap());
    let report = restarted_coordinator.collect(&sources, &clock).unwrap();

    assert_eq!(report.summary.today_tokens, 20);
    assert_eq!(report.accepted_event_count, 0);
}
