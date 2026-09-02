//! Persisted, privacy-safe provider rate-limit metadata.

use rusqlite::{params, types::Type, Transaction};

use crate::collection::RateLimitUpdate;
use crate::types::provider::Provider;
use crate::types::rate_limit::{ProviderRateLimitSummary, RateLimitSummary};

pub(crate) fn upsert(
    transaction: &Transaction<'_>,
    update: &RateLimitUpdate,
) -> rusqlite::Result<()> {
    let resets_at = i64::try_from(update.snapshot.resets_at).map_err(|_| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "rate-limit reset timestamp exceeds SQLite integer range",
        )))
    })?;
    transaction.execute(
        r#"
        INSERT INTO rate_limits (
            provider,
            window_minutes,
            used_percent,
            resets_at,
            observed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(provider, window_minutes) DO UPDATE SET
            used_percent = excluded.used_percent,
            resets_at = excluded.resets_at,
            observed_at = excluded.observed_at
        WHERE excluded.observed_at >= rate_limits.observed_at
        "#,
        params![
            update.provider.as_str(),
            i64::from(update.snapshot.window_minutes),
            i64::from(update.snapshot.used_percent),
            resets_at,
            update.snapshot.observed_at,
        ],
    )?;
    Ok(())
}

pub(crate) fn query(
    connection: &rusqlite::Connection,
) -> rusqlite::Result<Vec<ProviderRateLimitSummary>> {
    let mut statement = connection.prepare(
        "SELECT provider, window_minutes, used_percent, resets_at
         FROM rate_limits
         ORDER BY provider, window_minutes",
    )?;
    let rows = statement
        .query_map([], |row| {
            let provider_name: String = row.get(0)?;
            let provider = Provider::from_str(&provider_name)
                .ok_or_else(|| invalid_value(0, Type::Text, "unknown provider in rate_limits"))?;
            let window_minutes: u32 = row
                .get::<_, i64>(1)?
                .try_into()
                .map_err(|_| invalid_value(1, Type::Integer, "invalid rate-limit window"))?;
            let used_percent: u8 = row
                .get::<_, i64>(2)?
                .try_into()
                .map_err(|_| invalid_value(2, Type::Integer, "invalid rate-limit percent"))?;
            let resets_at: u64 = row
                .get::<_, i64>(3)?
                .try_into()
                .map_err(|_| invalid_value(3, Type::Integer, "invalid rate-limit reset"))?;
            Ok(ProviderRateLimitSummary {
                provider,
                rate_limit: RateLimitSummary {
                    window_minutes,
                    used_percent,
                    resets_at,
                },
            })
        })?
        .collect();
    rows
}

fn invalid_value(column: usize, value_type: Type, message: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        value_type,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )),
    )
}
