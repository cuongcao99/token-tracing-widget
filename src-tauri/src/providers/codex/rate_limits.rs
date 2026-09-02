//! Bounded extraction of Codex rate-limit metadata.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde_json::Value;

use crate::types::rate_limit::RateLimitSnapshot;
use crate::utils::windows_time::parse_timestamp_seconds;

const SUPPORTED_WINDOWS: [u32; 2] = [300, 10_080];
// ponytail: inspect the last 1 MiB; add a persisted source index only if rate metadata moves earlier than the tail.
const MAX_RATE_LIMIT_TAIL_BYTES: u64 = 1_048_576;

pub(crate) fn read_latest(file_path: &Path) -> Vec<RateLimitSnapshot> {
    let Ok(mut file) = File::open(file_path) else {
        return Vec::new();
    };
    let Ok(length) = file.metadata().map(|metadata| metadata.len()) else {
        return Vec::new();
    };
    let start = length.saturating_sub(MAX_RATE_LIMIT_TAIL_BYTES);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }

    let mut bytes = Vec::new();
    if file
        .take(MAX_RATE_LIMIT_TAIL_BYTES)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return Vec::new();
    }
    if start > 0 {
        let Some(newline) = bytes.iter().position(|byte| *byte == b'\n') else {
            return Vec::new();
        };
        bytes.drain(..=newline);
    }

    let mut latest: BTreeMap<u32, (i64, RateLimitSnapshot)> = BTreeMap::new();
    for line in bytes.split(|byte| *byte == b'\n') {
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        let Some(timestamp) = record.get("timestamp").and_then(Value::as_str) else {
            continue;
        };
        let Some(timestamp_seconds) = parse_timestamp_seconds(timestamp) else {
            continue;
        };
        let Some(rate_limits) = record
            .get("payload")
            .and_then(Value::as_object)
            .and_then(|payload| payload.get("rate_limits"))
            .and_then(Value::as_object)
        else {
            continue;
        };

        for key in ["primary", "secondary"] {
            let Some(snapshot) = parse_snapshot(rate_limits.get(key), timestamp) else {
                continue;
            };
            let should_replace = latest
                .get(&snapshot.window_minutes)
                .map_or(true, |(observed_seconds, _)| {
                    timestamp_seconds >= *observed_seconds
                });
            if should_replace {
                latest.insert(snapshot.window_minutes, (timestamp_seconds, snapshot));
            }
        }
    }

    latest.into_values().map(|(_, snapshot)| snapshot).collect()
}

fn parse_snapshot(value: Option<&Value>, observed_at: &str) -> Option<RateLimitSnapshot> {
    let object = value?.as_object()?;
    let window_minutes = u32::try_from(object.get("window_minutes")?.as_u64()?).ok()?;
    if !SUPPORTED_WINDOWS.contains(&window_minutes) {
        return None;
    }
    let used_percent = object.get("used_percent")?.as_f64()?;
    if !used_percent.is_finite() || !(0.0..=100.0).contains(&used_percent) {
        return None;
    }
    let resets_at = object.get("resets_at")?.as_u64()?;
    i64::try_from(resets_at).ok()?;

    Some(RateLimitSnapshot {
        window_minutes,
        used_percent: used_percent.round() as u8,
        resets_at,
        observed_at: observed_at.to_owned(),
    })
}
