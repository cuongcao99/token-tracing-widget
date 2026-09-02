//! Session table access.

use rusqlite::{params, OptionalExtension, Transaction};

use crate::types::provider::Provider;
use crate::types::session_usage_summary::normalize_session_name;
use crate::types::usage_event::UsageEvent;

#[derive(Debug, Clone)]
struct SessionRecord {
    started_at: String,
    last_activity_at: String,
    display_name: Option<String>,
    display_name_updated_at: Option<String>,
}

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

pub(crate) fn rekey(
    transaction: &Transaction<'_>,
    provider: Provider,
    old_session_key: &str,
    new_session_key: &str,
) -> rusqlite::Result<()> {
    if old_session_key == new_session_key {
        return Ok(());
    }

    let Some(old) = load(transaction, provider, old_session_key)? else {
        return Ok(());
    };
    let target = load(transaction, provider, new_session_key)?;
    let merged = merge_records(&old, target.as_ref());

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
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(provider, session_key) DO UPDATE SET
            started_at = excluded.started_at,
            last_activity_at = excluded.last_activity_at,
            display_name = excluded.display_name,
            display_name_updated_at = excluded.display_name_updated_at
        "#,
        params![
            provider.as_str(),
            new_session_key,
            merged.started_at,
            merged.last_activity_at,
            merged.display_name,
            merged.display_name_updated_at,
        ],
    )?;
    transaction.execute(
        "DELETE FROM sessions WHERE provider = ?1 AND session_key = ?2",
        params![provider.as_str(), old_session_key],
    )?;
    Ok(())
}

pub(crate) fn update_display_name(
    transaction: &Transaction<'_>,
    provider: Provider,
    session_key: &str,
    name: &str,
    updated_at: &str,
) -> rusqlite::Result<()> {
    let Some(name) = normalize_session_name(Some(name)) else {
        return Ok(());
    };
    transaction.execute(
        r#"
        UPDATE sessions
        SET display_name = ?3,
            display_name_updated_at = ?4
        WHERE provider = ?1
          AND session_key = ?2
          AND (
              display_name_updated_at IS NULL
              OR display_name_updated_at <= ?4
          )
        "#,
        params![provider.as_str(), session_key, name, updated_at],
    )?;
    Ok(())
}

fn load(
    transaction: &Transaction<'_>,
    provider: Provider,
    session_key: &str,
) -> rusqlite::Result<Option<SessionRecord>> {
    transaction
        .query_row(
            r#"
            SELECT started_at, last_activity_at, display_name, display_name_updated_at
            FROM sessions
            WHERE provider = ?1 AND session_key = ?2
            "#,
            params![provider.as_str(), session_key],
            |row| {
                Ok(SessionRecord {
                    started_at: row.get(0)?,
                    last_activity_at: row.get(1)?,
                    display_name: row.get(2)?,
                    display_name_updated_at: row.get(3)?,
                })
            },
        )
        .optional()
}

fn merge_records(old: &SessionRecord, target: Option<&SessionRecord>) -> SessionRecord {
    let Some(target) = target else {
        return old.clone();
    };
    let (display_name, display_name_updated_at) =
        match (old.display_name.as_ref(), target.display_name.as_ref()) {
            (Some(_), Some(_))
                if is_at_least_as_new(
                    old.display_name_updated_at.as_deref(),
                    target.display_name_updated_at.as_deref(),
                ) =>
            {
                (
                    old.display_name.clone(),
                    old.display_name_updated_at.clone(),
                )
            }
            (Some(_), Some(_)) => (
                target.display_name.clone(),
                target.display_name_updated_at.clone(),
            ),
            (Some(_), None) => (
                old.display_name.clone(),
                old.display_name_updated_at.clone(),
            ),
            (None, Some(_)) => (
                target.display_name.clone(),
                target.display_name_updated_at.clone(),
            ),
            (None, None) => (None, None),
        };

    SessionRecord {
        started_at: std::cmp::min(old.started_at.clone(), target.started_at.clone()),
        last_activity_at: std::cmp::max(
            old.last_activity_at.clone(),
            target.last_activity_at.clone(),
        ),
        display_name,
        display_name_updated_at,
    }
}

fn is_at_least_as_new(candidate: Option<&str>, current: Option<&str>) -> bool {
    match (candidate, current) {
        (Some(candidate), Some(current)) => candidate >= current,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => true,
    }
}
