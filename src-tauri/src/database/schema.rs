//! SQLite table definitions and migrations.

use rusqlite::Connection;

pub(crate) fn initialize(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS sources (
            provider TEXT PRIMARY KEY NOT NULL,
            configured_root TEXT NOT NULL,
            enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
            health_state TEXT NOT NULL,
            last_error_category TEXT,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
            provider TEXT NOT NULL,
            session_key TEXT NOT NULL,
            started_at TEXT NOT NULL,
            last_activity_at TEXT NOT NULL,
            display_name TEXT,
            display_name_updated_at TEXT,
            PRIMARY KEY (provider, session_key)
        );

        CREATE TABLE IF NOT EXISTS usage_events (
            event_id TEXT PRIMARY KEY NOT NULL,
            provider TEXT NOT NULL,
            file_identity TEXT NOT NULL,
            session_key TEXT NOT NULL,
            source_position INTEGER NOT NULL CHECK (source_position >= 0),
            observed_at TEXT NOT NULL,
            counter_kind TEXT NOT NULL,
            monotonic_segment INTEGER NOT NULL CHECK (monotonic_segment >= 0),
            input_tokens INTEGER CHECK (input_tokens IS NULL OR input_tokens >= 0),
            cached_input_tokens INTEGER CHECK (cached_input_tokens IS NULL OR cached_input_tokens >= 0),
            output_tokens INTEGER CHECK (output_tokens IS NULL OR output_tokens >= 0),
            total_tokens INTEGER NOT NULL CHECK (total_tokens >= 0)
        );

        CREATE INDEX IF NOT EXISTS usage_events_observed_at_idx
            ON usage_events (observed_at, source_position);

        CREATE TABLE IF NOT EXISTS file_checkpoints (
            file_identity TEXT PRIMARY KEY NOT NULL,
            provider TEXT NOT NULL,
            byte_offset INTEGER NOT NULL CHECK (byte_offset >= 0),
            size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
            modified_at_unix_ms INTEGER NOT NULL CHECK (modified_at_unix_ms >= 0),
            pending_offset INTEGER CHECK (pending_offset IS NULL OR pending_offset >= 0),
            monotonic_segment INTEGER NOT NULL CHECK (monotonic_segment >= 0),
            last_cumulative_input_tokens INTEGER CHECK (last_cumulative_input_tokens IS NULL OR last_cumulative_input_tokens >= 0),
            last_cumulative_cached_input_tokens INTEGER CHECK (last_cumulative_cached_input_tokens IS NULL OR last_cumulative_cached_input_tokens >= 0),
            last_cumulative_output_tokens INTEGER CHECK (last_cumulative_output_tokens IS NULL OR last_cumulative_output_tokens >= 0),
            last_cumulative_total_tokens INTEGER CHECK (last_cumulative_total_tokens IS NULL OR last_cumulative_total_tokens >= 0),
            CHECK (byte_offset <= size_bytes),
            CHECK (pending_offset IS NULL OR pending_offset <= size_bytes)
        );

        CREATE TABLE IF NOT EXISTS settings (
            setting_key TEXT PRIMARY KEY NOT NULL,
            setting_value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS diagnostics (
            provider TEXT NOT NULL,
            category TEXT NOT NULL,
            occurrence_count INTEGER NOT NULL CHECK (occurrence_count >= 0),
            last_occurred_at TEXT NOT NULL,
            PRIMARY KEY (provider, category)
        );
        "#,
    )?;

    let mut columns = connection.prepare("PRAGMA table_info(sessions)")?;
    let existing_columns: Vec<String> = columns
        .query_map([], |row| row.get(1))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(columns);

    if !existing_columns
        .iter()
        .any(|column| column == "display_name")
    {
        connection.execute("ALTER TABLE sessions ADD COLUMN display_name TEXT", [])?;
    }
    if !existing_columns
        .iter()
        .any(|column| column == "display_name_updated_at")
    {
        connection.execute(
            "ALTER TABLE sessions ADD COLUMN display_name_updated_at TEXT",
            [],
        )?;
    }

    Ok(())
}
