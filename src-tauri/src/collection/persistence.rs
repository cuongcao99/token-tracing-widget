use crate::types::file_checkpoint::FileCheckpoint;
use crate::types::provider::Provider;
use crate::types::rate_limit::RateLimitSnapshot;
use crate::types::usage_event::UsageEvent;
use crate::usage::summary::SummaryRows;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionBatch {
    pub events: Vec<UsageEvent>,
    pub checkpoints: Vec<FileCheckpoint>,
    pub session_key_updates: Vec<SessionKeyUpdate>,
    pub session_name_updates: Vec<SessionNameUpdate>,
    pub rate_limit_updates: Vec<RateLimitUpdate>,
    pub source_updates: Vec<SourceUpdate>,
    pub diagnostics: Vec<DiagnosticUpdate>,
}

impl CollectionBatch {
    pub fn new(events: Vec<UsageEvent>, checkpoints: Vec<FileCheckpoint>) -> Self {
        Self {
            events,
            checkpoints,
            session_key_updates: Vec::new(),
            session_name_updates: Vec::new(),
            rate_limit_updates: Vec::new(),
            source_updates: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionNameUpdate {
    pub provider: Provider,
    pub session_key: String,
    pub name: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionKeyUpdate {
    pub provider: Provider,
    pub file_identity: String,
    pub session_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitUpdate {
    pub provider: Provider,
    pub snapshot: RateLimitSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUpdate {
    pub provider: Provider,
    pub configured_root: String,
    pub enabled: bool,
    pub health_state: String,
    pub last_error_category: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticUpdate {
    pub provider: Provider,
    pub category: String,
    pub occurrence_count: u64,
    pub last_occurred_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionStoreError {
    Read,
    Write,
    InvalidValue,
}

impl std::fmt::Display for CollectionStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let category = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::InvalidValue => "invalid_value",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for CollectionStoreError {}

pub trait CollectionStore {
    fn load_checkpoint(
        &self,
        identity: &str,
    ) -> Result<Option<FileCheckpoint>, CollectionStoreError>;

    fn apply_batch(&mut self, batch: &CollectionBatch) -> Result<(), CollectionStoreError>;

    fn query_events_for_summary(
        &self,
        day_start: &str,
        now: &str,
    ) -> Result<SummaryRows, CollectionStoreError>;
}
