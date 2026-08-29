#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use serde_json::json;

    use super::{
        sanitize_fixture_record, validate_serialized, FixtureShape, PrivacyError,
        SourceStringLedger,
    };

    #[test]
    fn raw_strings_and_content_fields_never_reach_fixture_output() {
        let raw = json!({
            "type": "token_event",
            "session_id": "real-session-92af",
            "event_id": "real-event-b671",
            "timestamp": "2026-08-29T09:12:13Z",
            "usage": {"input_tokens": 1200, "output_tokens": 45},
            "message": {"content": "private prompt text"},
            "cwd": "C:\\Users\\person\\private-repository"
        });
        let mut ledger = SourceStringLedger::default();
        ledger.observe_value(&raw);
        let shape = FixtureShape {
            discriminator_path: Some("$.type".into()),
            discriminator_value: Some("token_event".into()),
            token_paths: vec![
                "$.usage.input_tokens".into(),
                "$.usage.output_tokens".into(),
            ],
            timestamp_path: Some("$.timestamp".into()),
            session_key_path: Some("$.session_id".into()),
            event_key_path: Some("$.event_id".into()),
        };

        let fixture = sanitize_fixture_record(&raw, &shape, 0).unwrap();
        let serialized = serde_json::to_string(&fixture).unwrap();
        validate_serialized(
            &serialized,
            &ledger,
            &BTreeSet::from(["token_event".to_string()]),
            Path::new(r"C:\Users\person"),
        )
        .unwrap();

        assert!(serialized.contains("session-synthetic-001"));
        assert!(serialized.contains("event-synthetic-001"));
        assert!(serialized.contains("2026-01-01T00:00:00Z"));
        assert!(!serialized.contains("real-session-92af"));
        assert!(!serialized.contains("real-event-b671"));
        assert!(!serialized.contains("private prompt text"));
        assert!(!serialized.contains("private-repository"));
        assert!(!serialized.contains("message"));
        assert!(!serialized.contains("cwd"));
    }

    #[test]
    fn rejects_absolute_paths() {
        let result = validate_serialized(
            r#"{"value":"C:\\Users\\person\\secret"}"#,
            &SourceStringLedger::default(),
            &BTreeSet::new(),
            Path::new(r"C:\Users\person"),
        );

        assert_eq!(result, Err(PrivacyError::AbsolutePath));
    }

    #[test]
    fn rejects_unc_paths() {
        let result = validate_serialized(
            r#"{"value":"\\\\server\\share\\secret"}"#,
            &SourceStringLedger::default(),
            &BTreeSet::new(),
            Path::new(r"C:\Users\person"),
        );

        assert_eq!(result, Err(PrivacyError::AbsolutePath));
    }

    #[test]
    fn rejects_uris() {
        let result = validate_serialized(
            r#"{"value":"https://example.invalid/private"}"#,
            &SourceStringLedger::default(),
            &BTreeSet::new(),
            Path::new(r"C:\Users\person"),
        );

        assert_eq!(result, Err(PrivacyError::Uri));
    }

    #[test]
    fn rejects_source_identifiers() {
        let mut ledger = SourceStringLedger::default();
        ledger.observe_value(&json!("real-session-92af"));
        let result = validate_serialized(
            r#"{"value":"real-session-92af"}"#,
            &ledger,
            &BTreeSet::new(),
            Path::new(r"C:\Users\person"),
        );

        assert_eq!(result, Err(PrivacyError::SourceStringLeak));
    }

    #[test]
    fn rejects_oversized_discriminator() {
        let value = "d".repeat(65);
        let serialized = serde_json::json!({"type": value}).to_string();
        let result = validate_serialized(
            &serialized,
            &SourceStringLedger::default(),
            &BTreeSet::new(),
            Path::new(r"C:\Users\person"),
        );

        assert_eq!(result, Err(PrivacyError::OversizedStructuralString));
    }

    #[test]
    fn rejects_negative_token_counter_before_conversion() {
        let raw = json!({"usage": {"input_tokens": -1}});
        let shape = FixtureShape {
            discriminator_path: None,
            discriminator_value: None,
            token_paths: vec!["$.usage.input_tokens".into()],
            timestamp_path: None,
            session_key_path: None,
            event_key_path: None,
        };

        let result = sanitize_fixture_record(&raw, &shape, 0);

        assert_eq!(result, Err(PrivacyError::InvalidTokenCounter));
    }

    #[test]
    fn rejects_negative_serialized_token_counter() {
        let result = validate_serialized(
            r#"{"usage":{"input_tokens":-1}}"#,
            &SourceStringLedger::default(),
            &BTreeSet::new(),
            Path::new(r"C:\Users\person"),
        );

        assert_eq!(result, Err(PrivacyError::InvalidTokenCounter));
    }

    #[test]
    fn rejects_content_field_even_without_source_string() {
        let result = validate_serialized(
            r#"{"message":{"content":true}}"#,
            &SourceStringLedger::default(),
            &BTreeSet::new(),
            Path::new(r"C:\Users\person"),
        );

        assert_eq!(result, Err(PrivacyError::SourceStringLeak));
    }
}
use std::collections::BTreeSet;
use std::fmt;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureShape {
    pub discriminator_path: Option<String>,
    pub discriminator_value: Option<String>,
    pub token_paths: Vec<String>,
    pub timestamp_path: Option<String>,
    pub session_key_path: Option<String>,
    pub event_key_path: Option<String>,
}

#[derive(Default)]
pub struct SourceStringLedger {
    hashes: BTreeSet<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyError {
    InvalidJson,
    SourceStringLeak,
    AbsolutePath,
    Uri,
    OversizedStructuralString,
    InvalidTokenCounter,
}

impl fmt::Display for PrivacyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::InvalidJson => "invalid_json",
            Self::SourceStringLeak => "source_string_leak",
            Self::AbsolutePath => "absolute_path",
            Self::Uri => "uri",
            Self::OversizedStructuralString => "oversized_structural_string",
            Self::InvalidTokenCounter => "invalid_token_counter",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for PrivacyError {}

impl SourceStringLedger {
    pub fn observe_value(&mut self, value: &Value) {
        match value {
            Value::String(string) => {
                self.hashes.insert(hash_string(string));
            }
            Value::Array(values) => {
                for value in values {
                    self.observe_value(value);
                }
            }
            Value::Object(fields) => {
                for value in fields.values() {
                    self.observe_value(value);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    pub fn contains(&self, value: &str) -> bool {
        self.hashes.contains(&hash_string(value))
    }
}

fn hash_string(value: &str) -> [u8; 32] {
    let digest = Sha256::digest(value.as_bytes());
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(&digest);
    hash
}

pub fn sanitize_fixture_record(
    raw: &Value,
    shape: &FixtureShape,
    ordinal: usize,
) -> Result<Value, PrivacyError> {
    let mut fields = Map::new();

    if let (Some(path), Some(value)) = (&shape.discriminator_path, &shape.discriminator_value) {
        if value.len() > 64 {
            return Err(PrivacyError::OversizedStructuralString);
        }
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(PrivacyError::InvalidJson);
        }
        insert_path(&mut fields, path, Value::String(value.clone()))?;
    }

    let synthetic_counter = synthetic_counter(ordinal)?;
    for path in &shape.token_paths {
        if let Some(raw_counter) = value_at_path(raw, path) {
            if raw_counter.as_u64().is_none() {
                return Err(PrivacyError::InvalidTokenCounter);
            }
        }
        insert_path(&mut fields, path, Value::from(synthetic_counter))?;
    }

    let synthetic_id = format!("{:03}", ordinal.saturating_add(1));
    if let Some(path) = &shape.timestamp_path {
        insert_path(
            &mut fields,
            path,
            Value::String(synthetic_timestamp(ordinal)?),
        )?;
    }
    if let Some(path) = &shape.session_key_path {
        insert_path(
            &mut fields,
            path,
            Value::String(format!("session-synthetic-{synthetic_id}")),
        )?;
    }
    if let Some(path) = &shape.event_key_path {
        insert_path(
            &mut fields,
            path,
            Value::String(format!("event-synthetic-{synthetic_id}")),
        )?;
    }

    fields.insert(
        "synthetic_unknown".to_string(),
        serde_json::json!({ "ignored": true }),
    );
    Ok(Value::Object(fields))
}

fn synthetic_counter(ordinal: usize) -> Result<u64, PrivacyError> {
    let ordinal = u64::try_from(ordinal).map_err(|_| PrivacyError::InvalidTokenCounter)?;
    ordinal
        .checked_mul(10)
        .and_then(|value| value.checked_add(10))
        .ok_or(PrivacyError::InvalidTokenCounter)
}

fn synthetic_timestamp(ordinal: usize) -> Result<String, PrivacyError> {
    let ordinal = u64::try_from(ordinal).map_err(|_| PrivacyError::InvalidTokenCounter)?;
    let days = ordinal / 86_400;
    let seconds = ordinal % 86_400;
    let base_days = days_from_civil(2026, 1, 1);
    let date = civil_from_days(
        base_days
            .checked_add(i64::try_from(days).map_err(|_| PrivacyError::InvalidTokenCounter)?)
            .ok_or(PrivacyError::InvalidTokenCounter)?,
    );
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let seconds = seconds % 60;
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z",
        year = date.0,
        month = date.1,
        day = date.2,
    ))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let days = days_since_epoch + 719_468;
    let era = (if days >= 0 { days } else { days - 146_096 }) / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

fn insert_path(
    target: &mut Map<String, Value>,
    path: &str,
    value: Value,
) -> Result<(), PrivacyError> {
    let segments = path_segments(path)?;
    insert_segments(target, &segments, value)
}

fn path_segments(path: &str) -> Result<Vec<&str>, PrivacyError> {
    let suffix = path.strip_prefix("$.").ok_or(PrivacyError::InvalidJson)?;
    if suffix.is_empty() {
        return Err(PrivacyError::InvalidJson);
    }
    let segments: Vec<_> = suffix.split('.').collect();
    if segments.iter().any(|segment| {
        segment.is_empty()
            || segment.len() > 64
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    }) {
        return Err(PrivacyError::InvalidJson);
    }
    Ok(segments)
}

fn value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let segments = path.strip_prefix("$.")?.split('.');
    let mut current = value;
    for segment in segments {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

fn insert_segments(
    target: &mut Map<String, Value>,
    segments: &[&str],
    value: Value,
) -> Result<(), PrivacyError> {
    let Some((segment, rest)) = segments.split_first() else {
        return Err(PrivacyError::InvalidJson);
    };
    if rest.is_empty() {
        target.insert((*segment).to_string(), value);
        return Ok(());
    }

    let entry = target
        .entry((*segment).to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(fields) = entry else {
        return Err(PrivacyError::InvalidJson);
    };
    insert_segments(fields, rest, value)
}

pub fn validate_serialized(
    serialized: &str,
    ledger: &SourceStringLedger,
    allowed_structural_values: &BTreeSet<String>,
    profile_root: &std::path::Path,
) -> Result<(), PrivacyError> {
    let value: Value = serde_json::from_str(serialized).map_err(|_| PrivacyError::InvalidJson)?;
    validate_value(&value, ledger, allowed_structural_values, profile_root)
}

fn validate_value(
    value: &Value,
    ledger: &SourceStringLedger,
    allowed_structural_values: &BTreeSet<String>,
    profile_root: &std::path::Path,
) -> Result<(), PrivacyError> {
    match value {
        Value::String(string) => {
            if string.len() > 64 {
                return Err(PrivacyError::OversizedStructuralString);
            }
            if contains_profile_root(string, profile_root) {
                return Err(PrivacyError::AbsolutePath);
            }
            if is_absolute_windows_path(string) || is_unc_path(string) {
                return Err(PrivacyError::AbsolutePath);
            }
            if is_uri(string) {
                return Err(PrivacyError::Uri);
            }
            if ledger.contains(string) && !allowed_structural_values.contains(string) {
                return Err(PrivacyError::SourceStringLeak);
            }
            if !allowed_structural_values.contains(string) && !is_synthetic_value(string) {
                return Err(PrivacyError::SourceStringLeak);
            }
        }
        Value::Array(values) => {
            for value in values {
                validate_value(value, ledger, allowed_structural_values, profile_root)?;
            }
        }
        Value::Object(fields) => {
            for (key, value) in fields {
                if is_forbidden_field(key) {
                    return Err(PrivacyError::SourceStringLeak);
                }
                if is_token_key(key) && value.is_number() && value.as_u64().is_none() {
                    return Err(PrivacyError::InvalidTokenCounter);
                }
                validate_value(value, ledger, allowed_structural_values, profile_root)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn is_token_key(key: &str) -> bool {
    key.ends_with("_tokens") || key.ends_with("Tokens")
}

fn is_forbidden_field(key: &str) -> bool {
    [
        "message",
        "content",
        "prompt",
        "response",
        "reasoning",
        "tool",
        "tools",
        "credential",
        "credentials",
        "cwd",
        "working_directory",
        "repository",
        "repo",
        "body",
        "raw",
        "source_record",
    ]
    .iter()
    .any(|candidate| key.eq_ignore_ascii_case(candidate))
}

fn is_synthetic_value(value: &str) -> bool {
    let is_synthetic_id = |prefix: &str| {
        value.strip_prefix(prefix).is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        })
    };
    is_synthetic_id("session-synthetic-")
        || is_synthetic_id("event-synthetic-")
        || (value.len() == 20
            && value.as_bytes()[4] == b'-'
            && value.as_bytes()[7] == b'-'
            && value.as_bytes()[10] == b'T'
            && value.as_bytes()[13] == b':'
            && value.as_bytes()[16] == b':'
            && value.ends_with('Z')
            && value.bytes().enumerate().all(|(index, byte)| {
                matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
            }))
}

fn contains_profile_root(value: &str, profile_root: &std::path::Path) -> bool {
    let profile = profile_root
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase();
    if profile.is_empty() || profile == "." {
        return false;
    }
    value
        .replace('\\', "/")
        .to_ascii_lowercase()
        .contains(&profile)
}

fn is_absolute_windows_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn is_unc_path(value: &str) -> bool {
    value.starts_with(r"\\") || value.starts_with("//")
}

fn is_uri(value: &str) -> bool {
    let Some(colon) = value.find(':') else {
        return false;
    };
    let scheme = &value[..colon];
    !scheme.is_empty()
        && scheme.as_bytes()[0].is_ascii_alphabetic()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}
