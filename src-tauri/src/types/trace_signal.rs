//! Privacy-safe lifecycle metadata received from a provider hook.

use std::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use super::provider::Provider;

pub const TRACE_SIGNAL_SCHEMA_VERSION: u32 = 1;
pub const MAX_OPAQUE_ID_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceLifecycle {
    StartOrContinue,
    Pause,
    Stop,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum ProviderEvent {
    SessionStart,
    UserPromptSubmit,
    Stop,
    StopFailure,
    SessionEnd,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceSignal {
    pub schema_version: u32,
    pub provider: Provider,
    pub lifecycle: TraceLifecycle,
    pub provider_event: ProviderEvent,
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opaque_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opaque_turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TraceSignalInput {
    schema_version: u32,
    provider: Provider,
    lifecycle: TraceLifecycle,
    provider_event: ProviderEvent,
    observed_at: String,
    opaque_session_id: Option<String>,
    opaque_turn_id: Option<String>,
    sequence: Option<u64>,
}

impl<'de> Deserialize<'de> for TraceSignal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = TraceSignalInput::deserialize(deserializer)?;
        Self::from_input(input).map_err(de::Error::custom)
    }
}

impl TraceSignal {
    fn from_input(input: TraceSignalInput) -> Result<Self, TraceSignalValidationError> {
        if input.schema_version != TRACE_SIGNAL_SCHEMA_VERSION {
            return Err(TraceSignalValidationError::UnsupportedSchemaVersion);
        }
        if !is_valid_timestamp(&input.observed_at) {
            return Err(TraceSignalValidationError::InvalidTimestamp);
        }
        if input
            .opaque_session_id
            .as_deref()
            .is_some_and(|value| !is_valid_opaque_id(value))
            || input
                .opaque_turn_id
                .as_deref()
                .is_some_and(|value| !is_valid_opaque_id(value))
        {
            return Err(TraceSignalValidationError::InvalidOpaqueId);
        }
        if !input
            .provider_event
            .matches(input.provider, input.lifecycle)
        {
            return Err(TraceSignalValidationError::InvalidProviderEvent);
        }

        Ok(Self {
            schema_version: input.schema_version,
            provider: input.provider,
            lifecycle: input.lifecycle,
            provider_event: input.provider_event,
            observed_at: input.observed_at,
            opaque_session_id: input.opaque_session_id,
            opaque_turn_id: input.opaque_turn_id,
            sequence: input.sequence,
        })
    }
}

impl ProviderEvent {
    fn matches(self, provider: Provider, lifecycle: TraceLifecycle) -> bool {
        matches!(
            (provider, self, lifecycle),
            (
                Provider::Claude,
                Self::UserPromptSubmit,
                TraceLifecycle::StartOrContinue
            ) | (Provider::Claude, Self::Stop, TraceLifecycle::Pause)
                | (Provider::Claude, Self::StopFailure, TraceLifecycle::Pause)
                | (Provider::Claude, Self::SessionEnd, TraceLifecycle::Stop)
                | (
                    Provider::Codex,
                    Self::SessionStart,
                    TraceLifecycle::StartOrContinue
                )
                | (
                    Provider::Codex,
                    Self::UserPromptSubmit,
                    TraceLifecycle::StartOrContinue
                )
                | (Provider::Codex, Self::Stop, TraceLifecycle::Pause)
                | (Provider::Codex, Self::SessionEnd, TraceLifecycle::Stop)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceSignalValidationError {
    UnsupportedSchemaVersion,
    InvalidTimestamp,
    InvalidOpaqueId,
    InvalidProviderEvent,
}

impl fmt::Display for TraceSignalValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnsupportedSchemaVersion => "unsupported_trace_signal_schema_version",
            Self::InvalidTimestamp => "invalid_trace_signal_timestamp",
            Self::InvalidOpaqueId => "invalid_trace_signal_opaque_id",
            Self::InvalidProviderEvent => "invalid_trace_signal_provider_event",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TraceSignalValidationError {}

fn is_valid_opaque_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_OPAQUE_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn is_valid_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !(20..=64).contains(&bytes.len())
        || !is_digits(&bytes[0..4])
        || bytes[4] != b'-'
        || !is_digits(&bytes[5..7])
        || bytes[7] != b'-'
        || !is_digits(&bytes[8..10])
        || bytes[10] != b'T'
        || !is_digits(&bytes[11..13])
        || bytes[13] != b':'
        || !is_digits(&bytes[14..16])
        || bytes[16] != b':'
        || !is_digits(&bytes[17..19])
    {
        return false;
    }

    let year = number(&bytes[0..4]);
    let month = number(&bytes[5..7]);
    let day = number(&bytes[8..10]);
    let hour = number(&bytes[11..13]);
    let minute = number(&bytes[14..16]);
    let second = number(&bytes[17..19]);
    if !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return false;
    }

    let mut timezone_start = 19;
    if bytes[timezone_start] == b'.' {
        timezone_start += 1;
        let fraction_start = timezone_start;
        while timezone_start < bytes.len() && bytes[timezone_start].is_ascii_digit() {
            timezone_start += 1;
        }
        if !(1..=9).contains(&(timezone_start - fraction_start)) {
            return false;
        }
    }

    match bytes.get(timezone_start..) {
        Some([b'Z']) => true,
        Some([sign, hour_tens, hour_ones, b':', minute_tens, minute_ones])
            if matches!(*sign, b'+' | b'-') =>
        {
            let timezone_hours = [*hour_tens, *hour_ones];
            let timezone_minutes = [*minute_tens, *minute_ones];
            if !is_digits(&timezone_hours) || !is_digits(&timezone_minutes) {
                return false;
            }
            number(&timezone_hours) <= 23 && number(&timezone_minutes) <= 59
        }
        _ => false,
    }
}

fn is_digits(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().all(|byte| byte.is_ascii_digit())
}

fn number(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(0, |value, byte| value * 10 + u32::from(*byte - b'0'))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}
