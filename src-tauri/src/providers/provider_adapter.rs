//! Shared interface implemented by Claude and Codex readers.

use std::fmt;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;

use crate::types::provider::Provider;
use crate::types::token_observation::TokenObservation;
use crate::utils::bounded_io;

pub const MAX_RECORD_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderReadObservation {
    pub observation: TokenObservation,
    pub source_position: u64,
}

impl ProviderReadObservation {
    pub fn new(observation: TokenObservation, source_position: u64) -> Self {
        Self {
            observation,
            source_position,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderReadResult {
    pub observations: Vec<ProviderReadObservation>,
    pub next_offset: u64,
    pub pending_offset: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderReadError {
    Io,
    InvalidJson,
    InvalidRecord,
    InvalidTokenCount,
    RecordTooLarge,
}

impl fmt::Display for ProviderReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::Io => "io",
            Self::InvalidJson => "invalid_json",
            Self::InvalidRecord => "invalid_record",
            Self::InvalidTokenCount => "invalid_token_count",
            Self::RecordTooLarge => "record_too_large",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for ProviderReadError {}

pub trait ProviderAdapter: Send + Sync {
    fn provider(&self) -> Provider;

    fn read_observations(
        &self,
        file: &Path,
        start_offset: u64,
    ) -> Result<ProviderReadResult, ProviderReadError>;
}

pub(crate) fn read_json_lines<F>(
    file_path: &Path,
    start_offset: u64,
    mut parse_record: F,
) -> Result<ProviderReadResult, ProviderReadError>
where
    F: FnMut(&serde_json::Value) -> Result<Option<TokenObservation>, ProviderReadError>,
{
    let file = File::open(file_path).map_err(|_| ProviderReadError::Io)?;
    let mut reader = BufReader::new(file);
    reader
        .seek(SeekFrom::Start(start_offset))
        .map_err(|_| ProviderReadError::Io)?;

    let mut observations = Vec::new();
    let mut next_offset = start_offset;
    let mut pending_offset = None;

    while let Some(line) =
        bounded_io::read_line(&mut reader, MAX_RECORD_BYTES).map_err(|error| {
            match error.kind() {
                std::io::ErrorKind::InvalidData => ProviderReadError::RecordTooLarge,
                _ => ProviderReadError::Io,
            }
        })?
    {
        let record_start = next_offset;
        let record_end = record_start.saturating_add(line.bytes.len() as u64);
        if line.bytes.iter().all(u8::is_ascii_whitespace) {
            next_offset = record_end;
            continue;
        }

        let record = match serde_json::from_slice(&line.bytes) {
            Ok(record) => record,
            Err(_) if !line.terminated => {
                pending_offset = Some(record_start);
                break;
            }
            Err(_) => return Err(ProviderReadError::InvalidJson),
        };
        if let Some(observation) = parse_record(&record)? {
            observations.push(ProviderReadObservation::new(observation, record_start));
        }
        next_offset = record_end;
    }

    Ok(ProviderReadResult {
        observations,
        next_offset,
        pending_offset,
    })
}
