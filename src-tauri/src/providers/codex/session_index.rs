//! Reading the bounded Codex session-name index.

use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::providers::provider_adapter::MAX_RECORD_BYTES;
use crate::types::session_usage_summary::normalize_session_name;
use crate::utils::windows_time::parse_timestamp_seconds;

const MAX_SESSION_INDEX_BYTES: u64 = 4 * 1024 * 1024;
// ponytail: bounded metadata-prefix scan; raise only if Codex moves session metadata later.
const MAX_SESSION_METADATA_BYTES: u64 = 64 * 1024;
const SESSION_ID_LENGTH: usize = 36;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionNameMetadata {
    pub(crate) name: String,
    pub(crate) updated_at: String,
}

pub(crate) fn session_name_for_file(file: &Path) -> Option<SessionNameMetadata> {
    let session_id = session_id_for_index(file)?;
    let index_path = session_index_path(file)?;
    lookup_name(&index_path, &session_id)
}

pub(crate) fn session_key_for_file(file: &Path) -> Option<String> {
    let session_id = session_id_for_index(file)?;
    let mut hasher = Sha256::new();
    hasher.update(b"codex-session:");
    hasher.update(session_id.to_ascii_lowercase().as_bytes());
    Some(format!("{:x}", hasher.finalize()))
}

pub(crate) fn is_indexed_session(file: &Path) -> bool {
    let Some(session_id) = session_id_for_index(file) else {
        return true;
    };
    let Some(index_path) = session_index_path(file) else {
        return true;
    };

    index_contains_session(&index_path, &session_id)
}

fn session_id_for_index(file: &Path) -> Option<String> {
    metadata_session_id_from_file(file).or_else(|| session_id_from_file(file))
}

fn metadata_session_id_from_file(file: &Path) -> Option<String> {
    let file = fs::File::open(file).ok()?;
    let mut reader = BufReader::new(file);
    let mut bytes_scanned = 0_u64;

    loop {
        let line = match crate::utils::bounded_io::read_line(&mut reader, MAX_RECORD_BYTES) {
            Ok(Some(line)) => line,
            Ok(None) | Err(_) => break,
        };
        bytes_scanned = bytes_scanned.saturating_add(line.bytes.len() as u64);
        if bytes_scanned > MAX_SESSION_METADATA_BYTES {
            break;
        }

        let Ok(record) = serde_json::from_slice::<Value>(&line.bytes) else {
            continue;
        };
        let Some(payload) = record.get("payload").and_then(Value::as_object) else {
            continue;
        };
        for key in ["session_id", "parent_thread_id"] {
            let Some(session_id) = payload.get(key).and_then(Value::as_str) else {
                continue;
            };
            if is_uuid(session_id) {
                return Some(session_id.to_owned());
            }
        }
    }

    None
}

fn lookup_name(index_path: &Path, session_id: &str) -> Option<SessionNameMetadata> {
    let contents = read_index(index_path)?;
    let mut latest: Option<(i64, SessionNameMetadata)> = None;
    for line in contents
        .lines()
        .filter(|line| line.len() <= MAX_RECORD_BYTES)
    {
        let record: Value = match serde_json::from_str(line) {
            Ok(record) => record,
            Err(_) => continue,
        };
        let Some(id) = record.get("id").and_then(Value::as_str) else {
            continue;
        };
        if !id.eq_ignore_ascii_case(session_id) {
            continue;
        }

        let Some(name) = normalize_session_name(record.get("thread_name").and_then(Value::as_str))
        else {
            continue;
        };
        let Some(updated_at) = record.get("updated_at").and_then(Value::as_str) else {
            continue;
        };
        let Some(updated_at_seconds) = parse_timestamp_seconds(updated_at) else {
            continue;
        };
        let candidate = SessionNameMetadata {
            name,
            updated_at: updated_at.to_owned(),
        };
        if latest
            .as_ref()
            .is_none_or(|(timestamp, _)| updated_at_seconds >= *timestamp)
        {
            latest = Some((updated_at_seconds, candidate));
        }
    }

    latest.map(|(_, metadata)| metadata)
}

fn index_contains_session(index_path: &Path, session_id: &str) -> bool {
    let Some(contents) = read_index(index_path) else {
        return false;
    };

    contents
        .lines()
        .filter(|line| line.len() <= MAX_RECORD_BYTES)
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .any(|record| {
            record
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.eq_ignore_ascii_case(session_id))
        })
}

fn read_index(index_path: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(index_path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_SESSION_INDEX_BYTES {
        return None;
    }

    let contents = String::from_utf8(fs::read(index_path).ok()?).ok()?;
    (contents.len() as u64 <= MAX_SESSION_INDEX_BYTES).then_some(contents)
}

fn session_index_path(file: &Path) -> Option<PathBuf> {
    let sessions_dir = file.ancestors().find(|ancestor| {
        ancestor
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("sessions"))
    })?;
    let codex_dir = sessions_dir.parent()?;
    if !codex_dir
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(".codex"))
    {
        return None;
    }
    Some(codex_dir.join("session_index.jsonl"))
}

fn session_id_from_file(file: &Path) -> Option<String> {
    let stem = file.file_stem()?.to_str()?;
    let start = stem.len().checked_sub(SESSION_ID_LENGTH)?;
    if stem.as_bytes().get(start.checked_sub(1)?) != Some(&b'-') {
        return None;
    }
    let candidate = stem.get(start..)?;
    if !is_uuid(candidate) {
        return None;
    }
    Some(candidate.to_owned())
}

fn is_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == SESSION_ID_LENGTH
        && bytes.iter().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                *byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}
