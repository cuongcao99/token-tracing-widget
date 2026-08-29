//! File checkpoint table access.

use rusqlite::{params, OptionalExtension, Transaction};

use crate::types::file_checkpoint::FileCheckpoint;
use crate::types::provider::Provider;

use super::checkpoints;

pub(crate) fn upsert(
    transaction: &Transaction<'_>,
    checkpoint: &FileCheckpoint,
) -> rusqlite::Result<()> {
    if checkpoints::validate(checkpoint).is_err() {
        return Err(rusqlite::Error::InvalidQuery);
    }

    transaction.execute(
        r#"
        INSERT INTO file_checkpoints (
            file_identity,
            provider,
            byte_offset,
            size_bytes,
            modified_at_unix_ms,
            pending_offset,
            monotonic_segment,
            last_cumulative_input_tokens,
            last_cumulative_cached_input_tokens,
            last_cumulative_output_tokens,
            last_cumulative_total_tokens
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(file_identity) DO UPDATE SET
            provider = excluded.provider,
            byte_offset = excluded.byte_offset,
            size_bytes = excluded.size_bytes,
            modified_at_unix_ms = excluded.modified_at_unix_ms,
            pending_offset = excluded.pending_offset,
            monotonic_segment = excluded.monotonic_segment,
            last_cumulative_input_tokens = excluded.last_cumulative_input_tokens,
            last_cumulative_cached_input_tokens = excluded.last_cumulative_cached_input_tokens,
            last_cumulative_output_tokens = excluded.last_cumulative_output_tokens,
            last_cumulative_total_tokens = excluded.last_cumulative_total_tokens
        "#,
        params![
            checkpoint.file_identity,
            checkpoint.provider.as_str(),
            to_sql_i64(checkpoint.byte_offset)?,
            to_sql_i64(checkpoint.size_bytes)?,
            to_sql_i64(checkpoint.modified_at_unix_ms)?,
            checkpoint.pending_offset.map(to_sql_i64).transpose()?,
            to_sql_i64(checkpoint.monotonic_segment)?,
            checkpoint
                .last_cumulative_input_tokens
                .map(to_sql_i64)
                .transpose()?,
            checkpoint
                .last_cumulative_cached_input_tokens
                .map(to_sql_i64)
                .transpose()?,
            checkpoint
                .last_cumulative_output_tokens
                .map(to_sql_i64)
                .transpose()?,
            checkpoint
                .last_cumulative_total_tokens
                .map(to_sql_i64)
                .transpose()?,
        ],
    )?;
    Ok(())
}

pub(crate) fn load(
    connection: &rusqlite::Connection,
    identity: &str,
) -> rusqlite::Result<Option<FileCheckpoint>> {
    connection
        .query_row(
            r#"
            SELECT
                file_identity,
                provider,
                byte_offset,
                size_bytes,
                modified_at_unix_ms,
                pending_offset,
                monotonic_segment,
                last_cumulative_input_tokens,
                last_cumulative_cached_input_tokens,
                last_cumulative_output_tokens,
                last_cumulative_total_tokens
            FROM file_checkpoints
            WHERE file_identity = ?1
            "#,
            [identity],
            |row| {
                let provider_name: String = row.get(1)?;
                let provider =
                    Provider::from_str(&provider_name).ok_or(rusqlite::Error::InvalidQuery)?;
                Ok(FileCheckpoint {
                    file_identity: row.get(0)?,
                    provider,
                    byte_offset: from_sql_i64(row.get(2)?)?,
                    size_bytes: from_sql_i64(row.get(3)?)?,
                    modified_at_unix_ms: from_sql_i64(row.get(4)?)?,
                    pending_offset: row
                        .get::<_, Option<i64>>(5)?
                        .map(from_sql_i64)
                        .transpose()?,
                    monotonic_segment: from_sql_i64(row.get(6)?)?,
                    last_cumulative_input_tokens: row
                        .get::<_, Option<i64>>(7)?
                        .map(from_sql_i64)
                        .transpose()?,
                    last_cumulative_cached_input_tokens: row
                        .get::<_, Option<i64>>(8)?
                        .map(from_sql_i64)
                        .transpose()?,
                    last_cumulative_output_tokens: row
                        .get::<_, Option<i64>>(9)?
                        .map(from_sql_i64)
                        .transpose()?,
                    last_cumulative_total_tokens: row
                        .get::<_, Option<i64>>(10)?
                        .map(from_sql_i64)
                        .transpose()?,
                    seen_event_ids: Vec::new(),
                })
            },
        )
        .optional()
}

fn to_sql_i64(value: u64) -> rusqlite::Result<i64> {
    i64::try_from(value).map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn from_sql_i64(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
