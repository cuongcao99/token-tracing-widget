//! Bounded sanitized diagnostic persistence.

use rusqlite::{params, Transaction};

use crate::collection::DiagnosticUpdate;

pub(crate) fn upsert(
    transaction: &Transaction<'_>,
    diagnostic: &DiagnosticUpdate,
) -> rusqlite::Result<()> {
    let occurrence_count = i64::try_from(diagnostic.occurrence_count)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    transaction.execute(
        r#"
        INSERT INTO diagnostics (provider, category, occurrence_count, last_occurred_at)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(provider, category) DO UPDATE SET
            occurrence_count = diagnostics.occurrence_count + excluded.occurrence_count,
            last_occurred_at = excluded.last_occurred_at
        "#,
        params![
            diagnostic.provider.as_str(),
            diagnostic.category,
            occurrence_count,
            diagnostic.last_occurred_at,
        ],
    )?;
    Ok(())
}
