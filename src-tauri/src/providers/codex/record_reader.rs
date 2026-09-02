//! Bounded reading of Codex JSONL session files.

use std::path::Path;

use super::record_parser::parse_record;
use crate::providers::provider_adapter::{
    read_json_lines, ProviderAdapter, ProviderReadError, ProviderReadResult,
};
use crate::types::provider::Provider;

#[derive(Debug, Default, Clone, Copy)]
pub struct CodexReader;

impl ProviderAdapter for CodexReader {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn read_observations(
        &self,
        file: &Path,
        start_offset: u64,
        max_source_bytes: u64,
    ) -> Result<ProviderReadResult, ProviderReadError> {
        read_json_lines(file, start_offset, max_source_bytes, parse_record)
    }
}
