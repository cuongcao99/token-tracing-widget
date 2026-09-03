//! Bounded reading of Codex JSONL session files.

use std::path::Path;

use super::record_parser::parse_record;
use super::session_index::{session_key_for_file, session_name_for_file};
use crate::providers::provider_adapter::{
    read_json_lines, ProviderAdapter, ProviderReadError, ProviderReadResult,
};
use crate::types::provider::Provider;
use crate::types::rate_limit::RateLimitSnapshot;

#[derive(Debug, Default, Clone, Copy)]
pub struct CodexReader;

impl ProviderAdapter for CodexReader {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn read_rate_limits(&self, file: &Path) -> Vec<RateLimitSnapshot> {
        super::rate_limits::read_latest(file)
    }

    fn should_read_file(&self, file: &Path, _local_day: &str) -> bool {
        super::session_index::is_indexed_session(file)
    }

    fn read_observations(
        &self,
        file: &Path,
        start_offset: u64,
        max_source_bytes: u64,
    ) -> Result<ProviderReadResult, ProviderReadError> {
        let session_key = session_key_for_file(file);
        let session_name = session_name_for_file(file);
        let mut result = read_json_lines(file, start_offset, max_source_bytes, parse_record)?;
        result.rate_limits = self.read_rate_limits(file);
        result.session_key = session_key.clone();
        if let Some(session_key) = session_key {
            for entry in &mut result.observations {
                entry.observation.source_session_key = Some(session_key.clone());
            }
        }
        if let Some(session_name) = session_name {
            for entry in &mut result.observations {
                entry.observation.session_name = Some(session_name.name.clone());
            }
            result.session_name = Some(session_name.name);
            result.session_name_updated_at = Some(session_name.updated_at);
        }
        Ok(result)
    }
}
