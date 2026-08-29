use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeOutcome {
    Detected,
    NotDetected,
    PermissionDenied,
    UnsupportedFormat,
    LimitReached,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FieldType {
    pub path: String,
    pub json_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RecordShape {
    pub discriminator_path: Option<String>,
    pub discriminator_value: Option<String>,
    pub field_types: Vec<FieldType>,
    pub counter_paths: Vec<String>,
    pub timestamp_path: Option<String>,
    pub session_key_path: Option<String>,
    pub event_key_path: Option<String>,
    pub sampled_record_count: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservedBehavior {
    PerEvent,
    Monotonic,
    ResetObserved,
    Uncertain,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CounterSequence {
    pub field_path: String,
    pub observed_behavior: ObservedBehavior,
    pub synthetic_values: Vec<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Coverage {
    pub files_considered: u64,
    pub complete_records_considered: u64,
    pub byte_limit_reached: bool,
    pub record_limit_reached: bool,
    pub supported_shape_found: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DiagnosticCount {
    pub category: String,
    pub count: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProviderReport {
    pub provider: Provider,
    pub outcome: ProbeOutcome,
    pub layout_patterns: Vec<String>,
    pub record_shapes: Vec<RecordShape>,
    pub counter_sequences: Vec<CounterSequence>,
    pub diagnostic_counts: Vec<DiagnosticCount>,
    pub coverage: Coverage,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProbeReport {
    pub schema_version: u32,
    pub providers: Vec<ProviderReport>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FixtureManifest {
    pub schema_version: u32,
    pub provider: Provider,
    pub outcome: ProbeOutcome,
    pub layout_patterns: Vec<String>,
    pub record_shapes: Vec<RecordShape>,
    pub counter_sequences: Vec<CounterSequence>,
    pub fixture_record_count: u64,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    use super::{FixtureManifest, Provider};
    use crate::sanitize::{validate_serialized, SourceStringLedger};

    #[test]
    fn committed_native_windows_fixtures_parse_and_pass_privacy_validation() {
        for provider in [Provider::Claude, Provider::Codex] {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("providers")
                .join(provider.as_str())
                .join("native_windows");
            let manifest_path = root.join("manifest.json");
            let manifest_json = fs::read_to_string(&manifest_path).unwrap();
            let manifest: FixtureManifest = serde_json::from_str(&manifest_json).unwrap();
            assert_eq!(manifest.provider, provider);

            let allowed = manifest_allowed_values(&manifest);
            validate_serialized(
                &manifest_json,
                &SourceStringLedger::default(),
                &allowed,
                Path::new(r"C:\synthetic-profile"),
            )
            .unwrap();

            let records_path = root.join("records.jsonl");
            let records = fs::read_to_string(&records_path).unwrap();
            let mut record_count = 0_u64;
            for line in records.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                serde_json::from_str::<serde_json::Value>(line).unwrap();
                validate_serialized(
                    line,
                    &SourceStringLedger::default(),
                    &allowed,
                    Path::new(r"C:\synthetic-profile"),
                )
                .unwrap();
                record_count = record_count.saturating_add(1);
            }
            assert_eq!(record_count, manifest.fixture_record_count);
        }
    }

    fn manifest_allowed_values(manifest: &FixtureManifest) -> BTreeSet<String> {
        let mut values = BTreeSet::new();
        values.insert(manifest.provider.as_str().to_string());
        values.insert(outcome_name(manifest.outcome).to_string());
        values.extend(manifest.layout_patterns.iter().cloned());
        for shape in &manifest.record_shapes {
            add_optional(&mut values, shape.discriminator_path.as_ref());
            add_optional(&mut values, shape.discriminator_value.as_ref());
            for field in &shape.field_types {
                values.insert(field.path.clone());
                values.insert(field.json_type.clone());
            }
            values.extend(shape.counter_paths.iter().cloned());
            add_optional(&mut values, shape.timestamp_path.as_ref());
            add_optional(&mut values, shape.session_key_path.as_ref());
            add_optional(&mut values, shape.event_key_path.as_ref());
        }
        for sequence in &manifest.counter_sequences {
            values.insert(sequence.field_path.clone());
            values.insert(behavior_name(sequence.observed_behavior).to_string());
        }
        values
    }

    fn add_optional(values: &mut BTreeSet<String>, value: Option<&String>) {
        if let Some(value) = value {
            values.insert(value.clone());
        }
    }

    fn outcome_name(outcome: super::ProbeOutcome) -> &'static str {
        match outcome {
            super::ProbeOutcome::Detected => "detected",
            super::ProbeOutcome::NotDetected => "not_detected",
            super::ProbeOutcome::PermissionDenied => "permission_denied",
            super::ProbeOutcome::UnsupportedFormat => "unsupported_format",
            super::ProbeOutcome::LimitReached => "limit_reached",
        }
    }

    fn behavior_name(behavior: super::ObservedBehavior) -> &'static str {
        match behavior {
            super::ObservedBehavior::PerEvent => "per_event",
            super::ObservedBehavior::Monotonic => "monotonic",
            super::ObservedBehavior::ResetObserved => "reset_observed",
            super::ObservedBehavior::Uncertain => "uncertain",
        }
    }
}
