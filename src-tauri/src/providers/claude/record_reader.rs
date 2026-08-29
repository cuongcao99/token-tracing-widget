//! Bounded reading of Claude JSONL session files.

use std::path::Path;

use super::record_parser::parse_record;
use crate::providers::provider_adapter::{
    read_json_lines, ProviderAdapter, ProviderReadError, ProviderReadResult,
};
use crate::types::provider::Provider;

#[derive(Debug, Default, Clone, Copy)]
pub struct ClaudeReader;

impl ProviderAdapter for ClaudeReader {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn read_observations(
        &self,
        file: &Path,
        start_offset: u64,
    ) -> Result<ProviderReadResult, ProviderReadError> {
        read_json_lines(file, start_offset, parse_record)
    }
}
