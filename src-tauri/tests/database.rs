use token_tracing_widget_lib::collection::{CollectionBatch, DiagnosticUpdate, SourceUpdate};
use token_tracing_widget_lib::database::connection::{IndexStore, StorageError};
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
