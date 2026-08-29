//! Session table access.

use rusqlite::{params, Transaction};

use crate::types::usage_event::UsageEvent;

pub(crate) fn upsert(transaction: &Transaction<'_>, event: &UsageEvent) -> rusqlite::Result<()> {
    transaction.execute(
        r#"
        INSERT INTO sessions (provider, session_key, started_at, last_activity_at)
        VALUES (?1, ?2, ?3, ?3)
        ON CONFLICT(provider, session_key) DO UPDATE SET
            started_at = CASE
                WHEN excluded.started_at < sessions.started_at
                THEN excluded.started_at
                ELSE sessions.started_at
            END,
            last_activity_at = CASE
                WHEN excluded.last_activity_at > sessions.last_activity_at
                THEN excluded.last_activity_at
                ELSE sessions.last_activity_at
            END
        "#,
        params![
            event.provider.as_str(),
            event.session_key,
            event.observed_at
        ],
    )?;
    Ok(())
}
