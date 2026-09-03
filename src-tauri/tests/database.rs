use std::path::PathBuf;

use token_tracing_widget_lib::collection::{
    CollectionBatch, DiagnosticUpdate, SessionNameUpdate, SourceUpdate,
};
use token_tracing_widget_lib::database::store::{IndexStore, StorageError};
use token_tracing_widget_lib::sources::source_config::SourceConfig;
use token_tracing_widget_lib::types::file_checkpoint::FileCheckpoint;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::token_observation::CounterKind;
use token_tracing_widget_lib::types::usage_event::UsageEvent;

fn test_usage_event(event_id: &str, file_identity: &str) -> UsageEvent {
    UsageEvent {
        event_id: event_id.to_owned(),
        provider: Provider::Claude,
        file_identity: file_identity.to_owned(),
        session_key: "session-a".to_owned(),
        session_name: None,
        source_position: 0,
        observed_at: "2026-01-01T00:00:00Z".to_owned(),
        counter_kind: CounterKind::Incremental,
        monotonic_segment: 0,
        input_tokens: Some(10),
        cached_input_tokens: Some(4),
        output_tokens: Some(10),
        total_tokens: 20,
    }
}

#[test]
fn event_and_checkpoint_commit_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = IndexStore::open(&directory.path().join("index.sqlite")).unwrap();
    let event = test_usage_event("event-1", "file-a");
    let checkpoint = FileCheckpoint::with_position("file-a", Provider::Claude, 42, 42);
    let batch = CollectionBatch::new(vec![event], vec![checkpoint]);

    database.apply_batch(&batch).unwrap();

    assert_eq!(database.count_usage_events().unwrap(), 1);
    assert_eq!(
        database
            .load_checkpoint("file-a")
            .unwrap()
            .unwrap()
            .byte_offset,
        42
    );
}

#[test]
fn failed_batch_rolls_back_event_and_checkpoint_together() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = IndexStore::open(&directory.path().join("index.sqlite")).unwrap();
    let event = test_usage_event("event-1", "file-new");
    let invalid_checkpoint = FileCheckpoint::with_position("file-new", Provider::Claude, 43, 42);
    let batch = CollectionBatch::new(vec![event], vec![invalid_checkpoint]);

    assert!(matches!(
        database.apply_batch(&batch),
        Err(StorageError::Write)
    ));
    assert_eq!(database.count_usage_events().unwrap(), 0);
    assert!(database.load_checkpoint("file-new").unwrap().is_none());
}

#[test]
fn summary_query_and_schema_expose_only_normalized_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("index.sqlite");
    let mut database = IndexStore::open(&database_path).unwrap();
    let batch = CollectionBatch::new(
        vec![test_usage_event("event-1", "file-a")],
        vec![FileCheckpoint::with_position(
            "file-a",
            Provider::Claude,
            42,
            42,
        )],
    );
    database.apply_batch(&batch).unwrap();

    let rows = database
        .query_events_for_summary("2026-01-01T00:00:00Z", "2026-01-01T00:00:01Z")
        .unwrap();
    assert_eq!(rows.events.len(), 1);
    assert_eq!(rows.events[0].total_tokens, 20);

    let connection = rusqlite::Connection::open(database_path).unwrap();
    let mut statement = connection
        .prepare("SELECT name, sql FROM sqlite_master WHERE type IN ('table', 'index')")
        .unwrap();
    let schema_text: String = statement
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let sql: Option<String> = row.get(1)?;
            Ok(format!("{name} {}", sql.unwrap_or_default()))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .join("\n")
        .to_ascii_lowercase();

    for forbidden in [
        "prompt",
        "response",
        "reasoning",
        "tool_payload",
        "credential",
        "repository",
        "working_directory",
        "raw_record",
    ] {
        assert!(
            !schema_text.contains(forbidden),
            "schema contains {forbidden}"
        );
    }
}

#[test]
fn summary_query_round_trips_only_the_session_display_name() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = IndexStore::open(&directory.path().join("index.sqlite")).unwrap();
    let mut event = test_usage_event("event-named", "file-a");
    event.session_name = Some("Run alpha".to_owned());

    database
        .apply_batch(&CollectionBatch::new(
            vec![event],
            vec![FileCheckpoint::with_position(
                "file-a",
                Provider::Claude,
                42,
                42,
            )],
        ))
        .unwrap();

    let rows = database
        .query_events_for_summary("2026-01-01T00:00:00Z", "2026-01-01T00:00:01Z")
        .unwrap();
    assert_eq!(rows.events[0].session_name.as_deref(), Some("Run alpha"));
}

#[test]
fn newer_session_name_wins_without_changing_identity_or_token_totals() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("index.sqlite");
    let mut database = IndexStore::open(&database_path).unwrap();

    let mut newer = test_usage_event("event-newer", "file-newer");
    newer.session_name = Some("Renamed run".to_owned());
    newer.observed_at = "2026-01-01T00:00:01Z".to_owned();
    newer.source_position = 2;
    database
        .apply_batch(&CollectionBatch::new(
            vec![newer],
            vec![FileCheckpoint::with_position(
                "file-newer",
                Provider::Claude,
                2,
                2,
            )],
        ))
        .unwrap();

    let mut older = test_usage_event("event-older", "file-older");
    older.session_name = Some("First run".to_owned());
    older.source_position = 1;
    database
        .apply_batch(&CollectionBatch::new(
            vec![older],
            vec![FileCheckpoint::with_position(
                "file-older",
                Provider::Claude,
                1,
                1,
            )],
        ))
        .unwrap();

    let rows = database
        .query_events_for_summary("2026-01-01T00:00:00Z", "2026-01-01T00:00:02Z")
        .unwrap();
    assert_eq!(rows.events.len(), 2);
    assert!(rows
        .events
        .iter()
        .all(|event| event.session_key == "session-a"));
    assert!(rows
        .events
        .iter()
        .all(|event| event.session_name.as_deref() == Some("Renamed run")));
    assert_eq!(
        rows.events
            .iter()
            .map(|event| event.total_tokens)
            .sum::<u64>(),
        40,
    );
}

#[test]
fn session_name_update_refreshes_a_persisted_session_without_new_tokens() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = IndexStore::open(&directory.path().join("index.sqlite")).unwrap();
    database
        .apply_batch(&CollectionBatch::new(
            vec![test_usage_event("event-1", "file-a")],
            vec![FileCheckpoint::with_position(
                "file-a",
                Provider::Claude,
                42,
                42,
            )],
        ))
        .unwrap();

    let mut batch = CollectionBatch::new(Vec::new(), Vec::new());
    batch.session_name_updates.push(SessionNameUpdate {
        provider: Provider::Claude,
        session_key: "session-a".to_owned(),
        name: "Renamed without tokens".to_owned(),
        updated_at: "2026-01-01T00:00:01Z".to_owned(),
    });
    database.apply_batch(&batch).unwrap();

    let rows = database
        .query_events_for_summary("2026-01-01T00:00:00Z", "2026-01-01T00:00:02Z")
        .unwrap();
    assert_eq!(rows.events.len(), 1);
    assert_eq!(
        rows.events[0].session_name.as_deref(),
        Some("Renamed without tokens")
    );
    assert_eq!(rows.events[0].total_tokens, 20);
}

#[test]
fn existing_sessions_gain_nullable_display_name_columns_without_data_loss() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("index.sqlite");
    let connection = rusqlite::Connection::open(&database_path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE sessions (
                provider TEXT NOT NULL,
                session_key TEXT NOT NULL,
                started_at TEXT NOT NULL,
                last_activity_at TEXT NOT NULL,
                PRIMARY KEY (provider, session_key)
            );
             INSERT INTO sessions (provider, session_key, started_at, last_activity_at)
             VALUES ('claude', 'legacy-session', '2026-01-01T00:00:00Z', '2026-01-01T00:00:01Z');",
        )
        .unwrap();
    drop(connection);

    let _database = IndexStore::open(&database_path).unwrap();
    let connection = rusqlite::Connection::open(database_path).unwrap();
    let columns: Vec<String> = connection
        .prepare("PRAGMA table_info(sessions)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(columns.iter().any(|column| column == "display_name"));
    assert!(columns
        .iter()
        .any(|column| column == "display_name_updated_at"));
    let legacy: (String, String) = connection
        .query_row(
            "SELECT started_at, last_activity_at FROM sessions WHERE session_key = 'legacy-session'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(legacy.0, "2026-01-01T00:00:00Z");
    assert_eq!(legacy.1, "2026-01-01T00:00:01Z");
}

#[test]
fn source_health_and_diagnostics_commit_with_the_same_batch() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = IndexStore::open(&directory.path().join("index.sqlite")).unwrap();
    let mut batch = CollectionBatch::new(Vec::new(), Vec::new());
    batch.source_updates.push(SourceUpdate {
        provider: Provider::Claude,
        configured_root: ".claude/projects".to_owned(),
        enabled: true,
        health_state: "detected".to_owned(),
        last_error_category: None,
        updated_at: "2026-01-01T00:00:00Z".to_owned(),
    });
    batch.diagnostics.push(DiagnosticUpdate {
        provider: Provider::Codex,
        category: "unavailable".to_owned(),
        occurrence_count: 1,
        last_occurred_at: "2026-01-01T00:00:00Z".to_owned(),
    });

    database.apply_batch(&batch).unwrap();

    let connection = rusqlite::Connection::open(directory.path().join("index.sqlite")).unwrap();
    let source_state: String = connection
        .query_row(
            "SELECT health_state FROM sources WHERE provider = ?1",
            ["claude"],
            |row| row.get(0),
        )
        .unwrap();
    let diagnostic_category: String = connection
        .query_row(
            "SELECT category FROM diagnostics WHERE provider = ?1",
            ["codex"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(source_state, "detected");
    assert_eq!(diagnostic_category, "unavailable");
}

#[test]
fn source_preferences_round_trip_and_remove_override() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("index.sqlite");
    let mut database = IndexStore::open(&path).unwrap();
    let config = SourceConfig::try_new(
        Provider::Claude,
        false,
        Some(PathBuf::from(r"C:\Users\tester\.claude\projects")),
    )
    .unwrap();

    database.save_source_config(&config).unwrap();
    assert_eq!(
        database
            .load_source_configs()
            .unwrap()
            .configs
            .get(Provider::Claude),
        &config
    );

    let automatic = SourceConfig::try_new(Provider::Claude, true, None).unwrap();
    database.save_source_config(&automatic).unwrap();
    assert_eq!(
        database
            .load_source_configs()
            .unwrap()
            .configs
            .get(Provider::Claude),
        &automatic
    );
}

#[test]
fn malformed_source_setting_defaults_only_its_provider() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("index.sqlite");
    let database = IndexStore::open(&path).unwrap();
    drop(database);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "INSERT INTO settings(setting_key, setting_value) VALUES (?1, ?2)",
            ["source.claude.enabled", "not-a-bool"],
        )
        .unwrap();
    drop(connection);

    let loaded = IndexStore::open(&path)
        .unwrap()
        .load_source_configs()
        .unwrap();
    assert!(loaded.configs.is_enabled(Provider::Claude));
    assert!(loaded.configs.is_enabled(Provider::Codex));
    assert_eq!(loaded.invalid_providers, vec![Provider::Claude]);
}
