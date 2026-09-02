//! Codex record shape and token-field parsing.

use serde_json::Value;

use crate::providers::provider_adapter::ProviderReadError;
use crate::types::provider::Provider;
use crate::types::token_observation::{CounterKind, TokenObservation};

pub fn parse_record(record: &Value) -> Result<Option<TokenObservation>, ProviderReadError> {
    let Some(payload) = record.get("payload").and_then(Value::as_object) else {
        return Ok(None);
    };

    if payload.get("type").and_then(Value::as_str) != Some("token_count") {
        return Ok(None);
    }

    let info = payload
        .get("info")
        .and_then(Value::as_object)
        .ok_or(ProviderReadError::InvalidRecord)?;
    let total_usage = info
        .get("total_token_usage")
        .and_then(Value::as_object)
        .ok_or(ProviderReadError::InvalidRecord)?;
    let input_tokens = required_token(total_usage.get("input_tokens"))?;
    let output_tokens = required_token(total_usage.get("output_tokens"))?;
    let total_tokens = match optional_token(total_usage.get("total_tokens"))? {
        Some(total) => total,
        None => input_tokens
            .checked_add(output_tokens)
            .ok_or(ProviderReadError::InvalidTokenCount)?,
    };

    Ok(Some(TokenObservation {
        provider: Provider::Codex,
        source_session_key: None,
        session_name: None,
        source_event_key: None,
        observed_at: required_string(record.get("timestamp"))?,
        counter_kind: CounterKind::Cumulative,
        input_tokens: Some(input_tokens),
        cached_input_tokens: optional_token(total_usage.get("cached_input_tokens"))?,
        output_tokens: Some(output_tokens),
        total_tokens,
    }))
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
