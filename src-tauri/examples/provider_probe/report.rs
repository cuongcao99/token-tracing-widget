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
