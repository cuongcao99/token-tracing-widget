use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;
use token_tracing_widget_lib::providers::claude::ClaudeReader;
use token_tracing_widget_lib::providers::codex::CodexReader;
use token_tracing_widget_lib::providers::provider_adapter::{ProviderAdapter, ProviderReadError};
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::token_observation::{CounterKind, TokenObservation};

fn fixture_path(provider: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("providers")
        .join(provider)
        .join("native_windows")
        .join("records.jsonl")
}

fn first_observation(
    observations: &[token_tracing_widget_lib::providers::provider_adapter::ProviderReadObservation],
) -> &TokenObservation {
    observations
        .first()
        .map(|entry| &entry.observation)
        .expect("fixture should contain one observation")
}

#[test]
fn claude_reader_returns_safe_incremental_observations() {
    let file = fixture_path("claude");
    let result = ClaudeReader::default()
        .read_observations(&file, 0)
        .expect("Claude fixture should be readable");
    let first = first_observation(&result.observations);

    assert_eq!(result.observations.len(), 30);
    assert_eq!(result.next_offset, fs::metadata(file).unwrap().len());
    assert_eq!(first.provider, Provider::Claude);
    assert_eq!(
        first.source_session_key.as_deref(),
        Some("session-synthetic-001")
    );
    assert_eq!(
        first.source_event_key.as_deref(),
        Some("event-synthetic-001")
    );
    assert_eq!(first.observed_at, "2026-01-01T00:00:00Z");
    assert_eq!(first.counter_kind, CounterKind::Incremental);
    assert_eq!(first.input_tokens, Some(10));
    assert_eq!(first.cached_input_tokens, Some(10));
    assert_eq!(first.output_tokens, Some(10));
    assert_eq!(first.total_tokens, 20);
}

#[test]
fn codex_reader_returns_safe_cumulative_observations() {
    let file = fixture_path("codex");
    let result = CodexReader::default()
        .read_observations(&file, 0)
        .expect("Codex fixture should be readable");
    let first = first_observation(&result.observations);

    assert_eq!(result.observations.len(), 6);
    assert_eq!(result.next_offset, fs::metadata(file).unwrap().len());
    assert_eq!(first.provider, Provider::Codex);
    assert!(first.source_session_key.is_none());
    assert!(first.source_event_key.is_none());
    assert_eq!(first.observed_at, "2026-01-01T00:00:00Z");
    assert_eq!(first.counter_kind, CounterKind::Cumulative);
    assert_eq!(first.input_tokens, Some(10));
    assert_eq!(first.cached_input_tokens, Some(10));
    assert_eq!(first.output_tokens, Some(10));
    assert_eq!(first.total_tokens, 20);
}

#[test]
fn readers_ignore_unknown_record_kinds() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"type":"unknown"}}"#).unwrap();
    writeln!(
        file,
        r#"{{"message":{{"id":"event-synthetic-001","type":"message","usage":{{"input_tokens":10,"output_tokens":10}}}},"sessionId":"session-synthetic-001","timestamp":"2026-01-01T00:00:00Z"}}"#
    )
    .unwrap();

    let result = ClaudeReader::default()
        .read_observations(file.path(), 0)
        .expect("unknown records should be ignored");

    assert_eq!(result.observations.len(), 1);
}

#[test]
fn readers_reject_negative_token_counts_without_exposing_record_data() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{"message":{{"id":"event-synthetic-001","type":"message","usage":{{"input_tokens":-1,"output_tokens":10}}}},"sessionId":"session-synthetic-001","timestamp":"2026-01-01T00:00:00Z"}}"#
    )
    .unwrap();

    let error = ClaudeReader::default()
        .read_observations(file.path(), 0)
        .expect_err("negative counters should fail closed");

    assert_eq!(error, ProviderReadError::InvalidTokenCount);
    assert_eq!(error.to_string(), "invalid_token_count");
}

#[test]
fn readers_reject_oversized_records_before_parsing() {
    let mut file = NamedTempFile::new().unwrap();
    let padding = "x".repeat(1_048_577);
    writeln!(file, r#"{{"type":"unknown","padding":"{padding}"}}"#).unwrap();

    let error = CodexReader::default()
        .read_observations(file.path(), 0)
        .expect_err("oversized records should fail closed");

    assert_eq!(error, ProviderReadError::RecordTooLarge);
}

#[test]
fn reader_resumes_from_a_saved_byte_offset() {
    let file = fixture_path("claude");
    let contents = fs::read(&file).unwrap();
    let first_line_end = contents
        .iter()
        .position(|byte| *byte == b'\n')
        .expect("fixture should contain a newline")
        + 1;

    let result = ClaudeReader::default()
        .read_observations(&file, first_line_end as u64)
        .expect("reader should resume at a line boundary");

    assert_eq!(result.observations.len(), 29);
    assert_eq!(
        result.observations[0]
            .observation
            .source_event_key
            .as_deref(),
        Some("event-synthetic-002")
    );
    assert_eq!(result.next_offset, contents.len() as u64);
    assert!(result.pending_offset.is_none());
}

#[test]
fn incomplete_final_line_stays_pending_until_completed() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        "{{\"message\":{{\"type\":\"message\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":10}}}},\"timestamp\":\"2026-01-01T00:00:00Z\"}}\n{{\"message\":"
    )
    .unwrap();

    let first = ClaudeReader::default()
        .read_observations(file.path(), 0)
        .expect("complete records must be readable");

    assert_eq!(first.observations.len(), 1);
    assert_eq!(first.next_offset, first.pending_offset.unwrap());

    write!(
        file,
        "{{\"type\":\"message\",\"usage\":{{\"input_tokens\":20,\"output_tokens\":20}}}},\"timestamp\":\"2026-01-01T00:00:01Z\"}}\n"
    )
    .unwrap();
    let second = ClaudeReader::default()
        .read_observations(file.path(), first.next_offset)
        .expect("completed record must be readable");

    assert_eq!(second.observations.len(), 1);
    assert!(second.pending_offset.is_none());
}
