//! Shared interface implemented by Claude and Codex readers.

use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use crate::types::provider::Provider;
use crate::types::token_observation::TokenObservation;
use crate::utils::bounded_io;

pub const MAX_RECORD_BYTES: usize = 1_048_576;
pub const MAX_SOURCE_BYTES_PER_ATTEMPT: u64 = 50 * 1024 * 1024;

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
    pub bytes_read: u64,
    pub skipped_oversized_records: usize,
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
        max_source_bytes: u64,
    ) -> Result<ProviderReadResult, ProviderReadError>;
}

pub(crate) fn read_json_lines<F>(
    file_path: &Path,
    start_offset: u64,
    max_source_bytes: u64,
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
    let mut skipped_oversized_records: usize = 0;

    loop {
        let bytes_read = reader
            .stream_position()
            .map_err(|_| ProviderReadError::Io)?
            .saturating_sub(start_offset);
        let remaining_source_bytes = max_source_bytes.saturating_sub(bytes_read);
        if remaining_source_bytes == 0 {
            break;
        }

        let next_chunk_length = {
            let available = reader.fill_buf().map_err(|_| ProviderReadError::Io)?;
            if available.is_empty() {
                break;
            }
            available
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(available.len(), |position| position + 1)
        };
        if next_chunk_length as u64 > remaining_source_bytes {
            break;
        }

        let line_limit = remaining_source_bytes.min(MAX_RECORD_BYTES as u64) as usize;
        let line = match bounded_io::read_line(&mut reader, line_limit) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                if remaining_source_bytes < MAX_RECORD_BYTES as u64 {
                    break;
                }
                if !bounded_io::discard_line(&mut reader).map_err(|_| ProviderReadError::Io)? {
                    return Err(ProviderReadError::RecordTooLarge);
                }
                next_offset = reader
                    .stream_position()
                    .map_err(|_| ProviderReadError::Io)?;
                skipped_oversized_records = skipped_oversized_records.saturating_add(1);
                continue;
            }
            Err(_) => return Err(ProviderReadError::Io),
        };

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
        bytes_read: reader
            .stream_position()
            .map_err(|_| ProviderReadError::Io)?
            .saturating_sub(start_offset),
        skipped_oversized_records,
    })
}
