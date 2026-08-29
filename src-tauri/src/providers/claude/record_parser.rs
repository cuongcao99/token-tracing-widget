//! Claude record shape and token-field parsing.

use serde_json::Value;

use crate::providers::provider_adapter::ProviderReadError;
use crate::types::provider::Provider;
use crate::types::token_observation::{CounterKind, TokenObservation};

pub fn parse_record(record: &Value) -> Result<Option<TokenObservation>, ProviderReadError> {
    let Some(message) = record.get("message").and_then(Value::as_object) else {
        return Ok(None);
    };

    if message.get("type").and_then(Value::as_str) != Some("message") {
        return Ok(None);
    }

    let usage = message
        .get("usage")
        .and_then(Value::as_object)
        .ok_or(ProviderReadError::InvalidRecord)?;
    let input_tokens = required_token(usage.get("input_tokens"))?;
    let output_tokens = required_token(usage.get("output_tokens"))?;
    let total_tokens = input_tokens
        .checked_add(output_tokens)
        .ok_or(ProviderReadError::InvalidTokenCount)?;

    Ok(Some(TokenObservation {
        provider: Provider::Claude,
        source_session_key: first_string(record, &["sessionId", "session_id"]),
        source_event_key: first_string_from_map(message, &["id"])
            .or_else(|| first_string(record, &["uuid"])),
        observed_at: required_string(record.get("timestamp"))?,
        counter_kind: CounterKind::Incremental,
        input_tokens: Some(input_tokens),
        cached_input_tokens: optional_token(usage.get("cache_read_input_tokens"))?,
        output_tokens: Some(output_tokens),
        total_tokens,
    }))
}

fn first_string(value: &Value, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        value
            .get(*name)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    })
}

fn first_string_from_map(value: &serde_json::Map<String, Value>, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        value
            .get(*name)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    })
}

fn required_string(value: Option<&Value>) -> Result<String, ProviderReadError> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
        .ok_or(ProviderReadError::InvalidRecord)
}

fn required_token(value: Option<&Value>) -> Result<u64, ProviderReadError> {
    optional_token(value)?.ok_or(ProviderReadError::InvalidRecord)
}

fn optional_token(value: Option<&Value>) -> Result<Option<u64>, ProviderReadError> {
    match value {
        None => Ok(None),
        Some(Value::Number(number)) => number
            .as_u64()
            .map(Some)
            .ok_or(ProviderReadError::InvalidTokenCount),
        Some(_) => Err(ProviderReadError::InvalidTokenCount),
    }
}
