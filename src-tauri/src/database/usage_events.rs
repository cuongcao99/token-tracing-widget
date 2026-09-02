//! Usage event table access.

use rusqlite::{params, Transaction};

use crate::types::provider::Provider;
use crate::types::token_observation::CounterKind;
use crate::types::usage_event::UsageEvent;

pub(crate) fn insert(transaction: &Transaction<'_>, event: &UsageEvent) -> rusqlite::Result<()> {
    transaction.execute(
        r#"
        INSERT OR IGNORE INTO usage_events (
            event_id,
            provider,
            file_identity,
            session_key,
            source_position,
            observed_at,
            counter_kind,
            monotonic_segment,
            input_tokens,
            cached_input_tokens,
            output_tokens,
            total_tokens
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
        params![
            event.event_id,
            event.provider.as_str(),
            event.file_identity,
            event.session_key,
            to_sql_i64(event.source_position)?,
            event.observed_at,
            counter_kind_name(event.counter_kind),
            to_sql_i64(event.monotonic_segment)?,
            event.input_tokens.map(to_sql_i64).transpose()?,
            event.cached_input_tokens.map(to_sql_i64).transpose()?,
            event.output_tokens.map(to_sql_i64).transpose()?,
            to_sql_i64(event.total_tokens)?,
        ],
    )?;
    Ok(())
}

pub(crate) fn query_between(
    connection: &rusqlite::Connection,
    day_start: &str,
    now: &str,
) -> rusqlite::Result<Vec<UsageEvent>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            usage_events.event_id,
            usage_events.provider,
            usage_events.file_identity,
            usage_events.session_key,
            usage_events.source_position,
            usage_events.observed_at,
            usage_events.counter_kind,
            usage_events.monotonic_segment,
            usage_events.input_tokens,
            usage_events.cached_input_tokens,
            usage_events.output_tokens,
            usage_events.total_tokens,
            sessions.display_name
        FROM usage_events
        LEFT JOIN sessions
            ON sessions.provider = usage_events.provider
            AND sessions.session_key = usage_events.session_key
        WHERE usage_events.observed_at >= ?1 AND usage_events.observed_at <= ?2
        ORDER BY usage_events.observed_at ASC, usage_events.source_position ASC, usage_events.event_id ASC
        "#,
    )?;
    let rows = statement.query_map(params![day_start, now], |row| {
        let provider_name: String = row.get(1)?;
        let provider = Provider::from_str(&provider_name).ok_or(rusqlite::Error::InvalidQuery)?;
        let counter_kind_name: String = row.get(6)?;
        let counter_kind =
            counter_kind_from_name(&counter_kind_name).ok_or(rusqlite::Error::InvalidQuery)?;
        Ok(UsageEvent {
            event_id: row.get(0)?,
            provider,
            file_identity: row.get(2)?,
            session_key: row.get(3)?,
            source_position: from_sql_i64(row.get(4)?)?,
            observed_at: row.get(5)?,
            counter_kind,
            monotonic_segment: from_sql_i64(row.get(7)?)?,
            input_tokens: row
                .get::<_, Option<i64>>(8)?
                .map(from_sql_i64)
                .transpose()?,
            cached_input_tokens: row
                .get::<_, Option<i64>>(9)?
                .map(from_sql_i64)
                .transpose()?,
            output_tokens: row
                .get::<_, Option<i64>>(10)?
                .map(from_sql_i64)
                .transpose()?,
            total_tokens: from_sql_i64(row.get(11)?)?,
            session_name: row.get(12)?,
        })
    })?;
    rows.collect()
}

fn counter_kind_name(kind: CounterKind) -> &'static str {
    match kind {
        CounterKind::Incremental => "incremental",
        CounterKind::Cumulative => "cumulative",
    }
}

fn counter_kind_from_name(name: &str) -> Option<CounterKind> {
    match name {
        "incremental" => Some(CounterKind::Incremental),
        "cumulative" => Some(CounterKind::Cumulative),
        _ => None,
    }
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
