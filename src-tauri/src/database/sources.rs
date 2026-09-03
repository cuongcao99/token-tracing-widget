//! Configured source table access.

use rusqlite::{params, Transaction};

use crate::collection::SourceUpdate;

pub(crate) fn upsert(transaction: &Transaction<'_>, source: &SourceUpdate) -> rusqlite::Result<()> {
    transaction.execute(
        r#"
        INSERT INTO sources (
            provider,
            configured_root,
            enabled,
            health_state,
            last_error_category,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(provider) DO UPDATE SET
            configured_root = excluded.configured_root,
            enabled = excluded.enabled,
            health_state = excluded.health_state,
            last_error_category = excluded.last_error_category,
            updated_at = excluded.updated_at
        "#,
        params![
            source.provider.as_str(),
            source.configured_root,
            i64::from(source.enabled),
            source.health_state,
            source.last_error_category,
            source.updated_at,
        ],
    )?;
    Ok(())
}
