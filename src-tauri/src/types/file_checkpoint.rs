//! A restart-safe position in a provider source file.

use super::provider::Provider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCheckpoint {
    pub provider: Provider,
    pub file_identity: String,
    pub byte_offset: u64,
    pub size_bytes: u64,
    pub modified_at_unix_ms: u64,
    pub pending_offset: Option<u64>,
    pub monotonic_segment: u64,
    pub last_cumulative_input_tokens: Option<u64>,
    pub last_cumulative_cached_input_tokens: Option<u64>,
    pub last_cumulative_output_tokens: Option<u64>,
    pub last_cumulative_total_tokens: Option<u64>,
    pub seen_event_ids: Vec<String>,
}

impl FileCheckpoint {
    pub fn new(file_identity: impl Into<String>, provider: Provider) -> Self {
        Self {
            provider,
            file_identity: file_identity.into(),
            byte_offset: 0,
            size_bytes: 0,
            modified_at_unix_ms: 0,
            pending_offset: None,
            monotonic_segment: 0,
            last_cumulative_input_tokens: None,
            last_cumulative_cached_input_tokens: None,
            last_cumulative_output_tokens: None,
            last_cumulative_total_tokens: None,
            seen_event_ids: Vec::new(),
        }
    }

    pub fn with_position(
        file_identity: impl Into<String>,
        provider: Provider,
        byte_offset: u64,
        size_bytes: u64,
    ) -> Self {
        Self {
            byte_offset,
            size_bytes,
            ..Self::new(file_identity, provider)
        }
    }

    pub fn with_file_metadata(mut self, size_bytes: u64, modified_at_unix_ms: u64) -> Self {
        self.size_bytes = size_bytes;
        self.modified_at_unix_ms = modified_at_unix_ms;
        self
    }

    pub fn is_compatible_with(&self, provider: Provider) -> bool {
        self.provider == provider
    }
}
