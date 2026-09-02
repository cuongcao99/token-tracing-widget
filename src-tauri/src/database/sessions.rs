//! Session table access.

use rusqlite::{params, Transaction};

use crate::types::session_usage_summary::normalize_session_name;
use crate::types::usage_event::UsageEvent;

pub(crate) fn upsert(transaction: &Transaction<'_>, event: &UsageEvent) -> rusqlite::Result<()> {
    let session_name = normalize_session_name(event.session_name.as_deref());
    transaction.execute(
        r#"
        INSERT INTO sessions (
            provider,
            session_key,
            started_at,
            last_activity_at,
            display_name,
            display_name_updated_at
        )
        VALUES (?1, ?2, ?3, ?3, ?4, CASE WHEN ?4 IS NULL THEN NULL ELSE ?3 END)
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
            END,
            display_name = CASE
                WHEN excluded.display_name IS NOT NULL
                    AND (
                        sessions.display_name_updated_at IS NULL
                        OR excluded.display_name_updated_at >= sessions.display_name_updated_at
                    )
                THEN excluded.display_name
                ELSE sessions.display_name
            END,
            display_name_updated_at = CASE
                WHEN excluded.display_name IS NOT NULL
                    AND (
                        sessions.display_name_updated_at IS NULL
                        OR excluded.display_name_updated_at >= sessions.display_name_updated_at
                    )
                THEN excluded.display_name_updated_at
                ELSE sessions.display_name_updated_at
            END
        "#,
        params![
            event.provider.as_str(),
            event.session_key,
            event.observed_at,
            session_name,
        ],
    )?;
    Ok(())
}
