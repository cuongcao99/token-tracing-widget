#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use tempfile::tempdir;

    use super::{inspect_candidates, inspect_jsonl_file, RecordControl, StopReason};
    use crate::discovery::{CandidateFile, DiscoveryResult, ProbeLimits, RootState};
    use crate::report::{ObservedBehavior, Provider};

    #[test]
    fn streams_complete_records_and_sanitizes_partial_input() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("session.jsonl");
        fs::write(
            &path,
            concat!(
                r#"{"type":"token_event","session_id":"private-a","event_id":"private-e1","timestamp":"2026-08-29T10:00:00Z","usage":{"input_tokens":100,"output_tokens":10},"message":{"content":"never emit this"}}"#,
                "\n",
                r#"{"type":"token_event","session_id":"private-a","event_id":"private-e2","timestamp":"2026-08-29T10:00:01Z","usage":{"input_tokens":150,"output_tokens":25}}"#,
                "\n",
                r#"{"type":"token_event""#,
            ),
        )
        .unwrap();

        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Claude,
                root_state: RootState::Readable,
                candidates: vec![CandidateFile {
                    path: path.clone(),
                    layout_pattern: "<file>.jsonl".to_string(),
                    size: fs::metadata(&path).unwrap().len(),
                }],
                selected_bytes: fs::metadata(&path).unwrap().len(),
                discovery_errors: 0,
            },
            directory.path(),
            ProbeLimits::default(),
        );

        assert_eq!(inspected.report.coverage.complete_records_considered, 2);
        assert_eq!(diagnostic_count(&inspected, "partial_final_line"), 1);
        assert_eq!(inspected.report.record_shapes.len(), 1);
        assert_eq!(
            inspected.report.record_shapes[0].counter_paths,
            vec!["$.usage.input_tokens", "$.usage.output_tokens"]
        );
        let input_sequence = inspected
            .report
            .counter_sequences
            .iter()
            .find(|sequence| sequence.field_path == "$.usage.input_tokens")
            .unwrap();
        assert_eq!(
            input_sequence.observed_behavior,
            ObservedBehavior::Monotonic
        );
        assert_eq!(inspected.fixtures.len(), 2);
        let fixtures = inspected
            .fixtures
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        assert!(!fixtures.contains("private-a"));
        assert!(!fixtures.contains("private-e1"));
        assert!(!fixtures.contains("never emit this"));
    }

    #[test]
    fn classifies_monotonic_reset_per_event_and_uncertain_sequences() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("sequences.jsonl");
        fs::write(
            &path,
            concat!(
                r#"{"type":"monotonic","usage":{"input_tokens":10}}"#,
                "\n",
                r#"{"type":"monotonic","usage":{"input_tokens":20}}"#,
                "\n",
                r#"{"type":"reset","usage":{"output_tokens":100}}"#,
                "\n",
                r#"{"type":"reset","usage":{"output_tokens":20}}"#,
                "\n",
                r#"{"type":"reset","usage":{"output_tokens":30}}"#,
                "\n",
                r#"{"type":"per_event","usage":{"cached_tokens":100}}"#,
                "\n",
                r#"{"type":"per_event","usage":{"cached_tokens":20}}"#,
                "\n",
                r#"{"type":"per_event","usage":{"cached_tokens":10}}"#,
                "\n",
                r#"{"type":"uncertain","usage":{"total_tokens":1}}"#,
                "\n",
            ),
        )
        .unwrap();

        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Codex,
                root_state: RootState::Readable,
                candidates: vec![CandidateFile {
                    path: path.clone(),
                    layout_pattern: "<file>.jsonl".to_string(),
                    size: fs::metadata(&path).unwrap().len(),
                }],
                selected_bytes: fs::metadata(&path).unwrap().len(),
                discovery_errors: 0,
            },
            directory.path(),
            ProbeLimits::default(),
        );

        assert_eq!(
            behavior_for(&inspected, "monotonic"),
            ObservedBehavior::Monotonic
        );
        assert_eq!(
            behavior_for(&inspected, "reset"),
            ObservedBehavior::ResetObserved
        );
        assert_eq!(
            behavior_for(&inspected, "per_event"),
            ObservedBehavior::PerEvent
        );
        assert_eq!(
            behavior_for(&inspected, "uncertain"),
            ObservedBehavior::Uncertain
        );
    }

    #[test]
    fn samples_a_json_object_without_a_trailing_newline() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("session.json");
        fs::write(
            &path,
            r#"{"type":"token_event","usage":{"input_tokens":10}}"#,
        )
        .unwrap();

        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Claude,
                root_state: RootState::Readable,
                candidates: vec![CandidateFile {
                    path: path.clone(),
                    layout_pattern: "<file>.json".to_string(),
                    size: fs::metadata(&path).unwrap().len(),
                }],
                selected_bytes: fs::metadata(&path).unwrap().len(),
                discovery_errors: 0,
            },
            directory.path(),
            ProbeLimits::default(),
        );

        assert_eq!(inspected.report.coverage.complete_records_considered, 1);
        assert_eq!(
            inspected.report.outcome,
            crate::report::ProbeOutcome::Detected
        );
    }

    #[test]
    fn stops_at_record_limit_without_discarding_prior_records() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("limited.jsonl");
        fs::write(
            &path,
            concat!(
                r#"{"type":"token_event","usage":{"input_tokens":10}}"#,
                "\n",
                r#"{"type":"token_event","usage":{"input_tokens":20}}"#,
                "\n",
            ),
        )
        .unwrap();

        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Claude,
                root_state: RootState::Readable,
                candidates: vec![CandidateFile {
                    path: path.clone(),
                    layout_pattern: "<file>.jsonl".to_string(),
                    size: fs::metadata(&path).unwrap().len(),
                }],
                selected_bytes: fs::metadata(&path).unwrap().len(),
                discovery_errors: 0,
            },
            directory.path(),
            ProbeLimits {
                max_records: 1,
                ..ProbeLimits::default()
            },
        );

        assert_eq!(inspected.report.coverage.complete_records_considered, 1);
        assert!(inspected.report.coverage.record_limit_reached);
        assert_eq!(
            inspected.report.outcome,
            crate::report::ProbeOutcome::LimitReached
        );
        assert_eq!(inspected.fixtures.len(), 1);
    }

    #[test]
    fn reports_a_small_unterminated_jsonl_record_as_partial() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("partial.jsonl");
        fs::write(
            &path,
            r#"{"type":"token_event","usage":{"input_tokens":10}}"#,
        )
        .unwrap();

        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Claude,
                root_state: RootState::Readable,
                candidates: vec![CandidateFile {
                    path: path.clone(),
                    layout_pattern: "<file>.jsonl".to_string(),
                    size: fs::metadata(&path).unwrap().len(),
                }],
                selected_bytes: fs::metadata(&path).unwrap().len(),
                discovery_errors: 0,
            },
            directory.path(),
            ProbeLimits::default(),
        );

        assert_eq!(inspected.report.coverage.complete_records_considered, 0);
        assert_eq!(diagnostic_count(&inspected, "partial_final_line"), 1);
        assert_eq!(inspected.fixtures.len(), 0);
    }

    #[test]
    fn honors_the_total_jsonl_byte_cap_when_a_candidate_is_larger() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("byte-cap.jsonl");
        let first = br#"{"type":"token_event","usage":{"input_tokens":10}}"#;
        let second = br#"{"type":"token_event","usage":{"input_tokens":20}}"#;
        let mut contents = Vec::new();
        contents.extend_from_slice(first);
        contents.push(b'\n');
        contents.extend_from_slice(second);
        contents.push(b'\n');
        fs::write(&path, contents).unwrap();

        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Claude,
                root_state: RootState::Readable,
                candidates: vec![CandidateFile {
                    path,
                    layout_pattern: "<file>.jsonl".to_string(),
                    size: u64::MAX,
                }],
                selected_bytes: u64::MAX,
                discovery_errors: 0,
            },
            directory.path(),
            ProbeLimits {
                max_bytes: (first.len() + 1 + second.len() / 2) as u64,
                ..ProbeLimits::default()
            },
        );

        assert_eq!(inspected.report.coverage.complete_records_considered, 1);
        assert!(inspected.report.coverage.byte_limit_reached);
        assert_eq!(diagnostic_count(&inspected, "partial_final_line"), 0);
    }

    #[test]
    fn stops_jsonl_reading_after_the_record_callback_requests_it() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("callback.jsonl");
        let first = br#"{"type":"token_event","usage":{"input_tokens":10}}"#;
        let oversized = vec![b'x'; 128 * 1024];
        let mut contents = Vec::new();
        contents.extend_from_slice(first);
        contents.push(b'\n');
        contents.extend_from_slice(&oversized);
        fs::write(&path, contents).unwrap();

        let mut bytes_consumed = 0;
        let mut diagnostics = BTreeMap::new();
        let result = inspect_jsonl_file(
            &path,
            &mut bytes_consumed,
            &mut diagnostics,
            ProbeLimits {
                max_bytes: u64::MAX,
                max_record_bytes: 64,
                ..ProbeLimits::default()
            },
            |_, _| RecordControl::StopSufficientEvidence,
        );

        assert!(matches!(
            result.stop_reason,
            Some(StopReason::SufficientEvidence)
        ));
        assert_eq!(bytes_consumed, (first.len() + 1) as u64);
        assert_eq!(diagnostic_count_map(&diagnostics, "record_too_large"), 0);
    }

    #[test]
    fn preserves_a_bounded_discovery_error_diagnostic() {
        let directory = tempdir().unwrap();
        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Codex,
                root_state: RootState::Error,
                candidates: Vec::new(),
                selected_bytes: 0,
                discovery_errors: 7,
            },
            directory.path(),
            ProbeLimits::default(),
        );

        assert_eq!(diagnostic_count(&inspected, "discovery_error"), 7);
        assert_eq!(
            inspected.report.outcome,
            crate::report::ProbeOutcome::UnsupportedFormat
        );
    }

    #[test]
    fn caps_an_oversized_unterminated_line_before_parsing() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("oversized.jsonl");
        let oversized = vec![b'x'; 128 * 1024];
        fs::write(&path, oversized).unwrap();

        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Claude,
                root_state: RootState::Readable,
                candidates: vec![CandidateFile {
                    path: path.clone(),
                    layout_pattern: "<file>.jsonl".to_string(),
                    size: fs::metadata(&path).unwrap().len(),
                }],
                selected_bytes: fs::metadata(&path).unwrap().len(),
                discovery_errors: 0,
            },
            directory.path(),
            ProbeLimits {
                max_record_bytes: 32,
                max_bytes: 128 * 1024 + 1,
                ..ProbeLimits::default()
            },
        );

        assert_eq!(inspected.report.coverage.complete_records_considered, 0);
        assert_eq!(diagnostic_count(&inspected, "record_too_large"), 1);
        assert_eq!(diagnostic_count(&inspected, "partial_final_line"), 0);
    }

    #[test]
    fn stops_before_older_candidates_after_sufficient_counter_evidence() {
        let directory = tempdir().unwrap();
        let newest = directory.path().join("newest.jsonl");
        let older = directory.path().join("older.jsonl");
        fs::write(
            &newest,
            concat!(
                r#"{"type":"token_event","usage":{"input_tokens":10}}"#,
                "\n",
                r#"{"type":"token_event","usage":{"input_tokens":20}}"#,
                "\n",
            ),
        )
        .unwrap();
        fs::write(&older, b"not-json\n").unwrap();

        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Codex,
                root_state: RootState::Readable,
                candidates: vec![
                    CandidateFile {
                        path: newest.clone(),
                        layout_pattern: "<file>.jsonl".to_string(),
                        size: fs::metadata(&newest).unwrap().len(),
                    },
                    CandidateFile {
                        path: older.clone(),
                        layout_pattern: "<file>.jsonl".to_string(),
                        size: fs::metadata(&older).unwrap().len(),
                    },
                ],
                selected_bytes: fs::metadata(&newest).unwrap().len()
                    + fs::metadata(&older).unwrap().len(),
                discovery_errors: 0,
            },
            directory.path(),
            ProbeLimits::default(),
        );

        assert_eq!(inspected.report.coverage.files_considered, 1);
        assert_eq!(diagnostic_count(&inspected, "malformed_record"), 0);
    }

    #[test]
    fn keeps_emitted_fixtures_bounded_for_many_source_shapes() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("many-shapes.jsonl");
        let mut records = String::new();
        for index in 0..100 {
            records.push_str(&format!(
                "{{\"type\":\"shape-{index}\",\"usage\":{{\"input_tokens\":{index}}}}}\n"
            ));
        }
        fs::write(&path, records).unwrap();

        let inspected = inspect_candidates(
            DiscoveryResult {
                provider: Provider::Claude,
                root_state: RootState::Readable,
                candidates: vec![CandidateFile {
                    path: path.clone(),
                    layout_pattern: "<file>.jsonl".to_string(),
                    size: fs::metadata(&path).unwrap().len(),
                }],
                selected_bytes: fs::metadata(&path).unwrap().len(),
                discovery_errors: 0,
            },
            directory.path(),
            ProbeLimits {
                max_records: 100,
                ..ProbeLimits::default()
            },
        );

        assert!(inspected.fixtures.len() <= super::MAX_TOTAL_FIXTURES);
        assert!(inspected.fixtures.len() < 100);
    }

    fn behavior_for(inspected: &super::InspectedProvider, discriminator: &str) -> ObservedBehavior {
        inspected
            .report
            .record_shapes
            .iter()
            .find(|shape| shape.discriminator_value.as_deref() == Some(discriminator))
            .and_then(|shape| {
                inspected
                    .report
                    .counter_sequences
                    .iter()
                    .find(|sequence| sequence.field_path == shape.counter_paths[0])
            })
            .map(|sequence| sequence.observed_behavior)
            .unwrap()
    }

    fn diagnostic_count(inspected: &super::InspectedProvider, category: &str) -> u64 {
        inspected
            .report
            .diagnostic_counts
            .iter()
            .find(|diagnostic| diagnostic.category == category)
            .map_or(0, |diagnostic| diagnostic.count)
    }

    fn diagnostic_count_map(diagnostics: &BTreeMap<String, u64>, category: &str) -> u64 {
        diagnostics.get(category).copied().unwrap_or(0)
    }
}
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use serde_json::Value;

use crate::discovery::{DiscoveryResult, ProbeLimits, RootState};
use crate::report::{
    CounterSequence, Coverage, DiagnosticCount, FieldType, FixtureManifest, ObservedBehavior,
    ProbeOutcome, ProviderReport, RecordShape,
};
use crate::sanitize::{sanitize_fixture_record, FixtureShape, PrivacyError, SourceStringLedger};

const MAX_TOTAL_FIXTURES: usize = 32;

pub struct InspectedProvider {
    pub report: ProviderReport,
    pub manifest: FixtureManifest,
    pub fixtures: Vec<Value>,
    pub ledger: SourceStringLedger,
    pub allowed_structural_values: BTreeSet<String>,
}

pub fn inspect_candidates(
    discovery: DiscoveryResult,
    _profile_root: &Path,
    limits: ProbeLimits,
) -> InspectedProvider {
    let _selected_bytes = discovery.selected_bytes;
    let provider = discovery.provider;
    let mut ledger = SourceStringLedger::default();
    let mut allowed_structural_values = BTreeSet::new();
    let mut diagnostics = BTreeMap::new();
    if discovery.discovery_errors > 0 {
        diagnostics.insert("discovery_error".to_string(), discovery.discovery_errors);
    }
    let mut coverage = Coverage {
        files_considered: 0,
        complete_records_considered: 0,
        ..Coverage::default()
    };
    let mut layout_patterns = BTreeSet::new();
    let mut groups: BTreeMap<GroupKey, GroupEvidence> = BTreeMap::new();
    let mut fixtures = Vec::new();
    let mut bytes_consumed = 0_u64;
    let mut stop = false;

    let root_outcome = match discovery.root_state {
        RootState::Readable => None,
        RootState::NotDetected => Some(ProbeOutcome::NotDetected),
        RootState::PermissionDenied => Some(ProbeOutcome::PermissionDenied),
        RootState::Error => Some(ProbeOutcome::UnsupportedFormat),
    };

    if root_outcome.is_none() {
        for candidate in discovery.candidates {
            if stop {
                break;
            }
            coverage.files_considered += 1;
            layout_patterns.insert(candidate.layout_pattern.clone());
            if bytes_consumed >= limits.max_bytes {
                coverage.byte_limit_reached = true;
                break;
            }
            let is_json = candidate
                .path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
            let file_result = if is_json {
                let result = inspect_json_file(
                    &candidate.path,
                    &mut bytes_consumed,
                    &mut diagnostics,
                    limits,
                );
                for raw in result.values {
                    if coverage.complete_records_considered >= limits.max_records {
                        coverage.record_limit_reached = true;
                        stop = true;
                        break;
                    }
                    process_record(
                        raw,
                        &mut coverage,
                        &mut ledger,
                        &mut allowed_structural_values,
                        &mut groups,
                        &mut fixtures,
                        &mut diagnostics,
                    );
                }
                FileInspectionResult {
                    values: Vec::new(),
                    stop_reason: result.stop_reason,
                }
            } else {
                let remaining_records = limits
                    .max_records
                    .saturating_sub(coverage.complete_records_considered);
                inspect_jsonl_file(
                    &candidate.path,
                    &mut bytes_consumed,
                    &mut diagnostics,
                    ProbeLimits {
                        max_records: remaining_records,
                        ..limits
                    },
                    |raw, diagnostics| {
                        process_record(
                            raw,
                            &mut coverage,
                            &mut ledger,
                            &mut allowed_structural_values,
                            &mut groups,
                            &mut fixtures,
                            diagnostics,
                        );
                        RecordControl::Continue
                    },
                )
            };

            if let Some(reason) = file_result.stop_reason {
                match reason {
                    StopReason::ByteLimit => {
                        coverage.byte_limit_reached = true;
                        stop = true;
                    }
                    StopReason::RecordLimit => {
                        coverage.record_limit_reached = true;
                        stop = true;
                    }
                    StopReason::Io => {
                        increment(&mut diagnostics, "read_error");
                    }
                    StopReason::SufficientEvidence => stop = true,
                }
            }
            if !stop && has_sufficient_evidence(&groups) {
                stop = true;
            }
        }
    }

    coverage.supported_shape_found = !groups.is_empty();
    let outcome = root_outcome.unwrap_or_else(|| {
        if coverage.byte_limit_reached || coverage.record_limit_reached {
            ProbeOutcome::LimitReached
        } else if coverage.supported_shape_found {
            ProbeOutcome::Detected
        } else {
            ProbeOutcome::UnsupportedFormat
        }
    });

    let record_shapes = groups
        .values()
        .map(|group| group.record_shape())
        .collect::<Vec<_>>();
    let counter_sequences = groups
        .values()
        .flat_map(GroupEvidence::counter_sequences)
        .collect::<Vec<_>>();
    let layout_patterns = layout_patterns.into_iter().collect::<Vec<_>>();
    let diagnostic_counts = diagnostics
        .into_iter()
        .map(|(category, count)| DiagnosticCount { category, count })
        .collect::<Vec<_>>();
    let report = ProviderReport {
        provider,
        outcome,
        layout_patterns: layout_patterns.clone(),
        record_shapes: record_shapes.clone(),
        counter_sequences: counter_sequences.clone(),
        diagnostic_counts,
        coverage,
    };
    let manifest = FixtureManifest {
        schema_version: 1,
        provider,
        outcome,
        layout_patterns,
        record_shapes,
        counter_sequences,
        fixture_record_count: fixtures.len() as u64,
    };
    InspectedProvider {
        report,
        manifest,
        fixtures,
        ledger,
        allowed_structural_values,
    }
}

fn process_record(
    raw: Value,
    coverage: &mut Coverage,
    ledger: &mut SourceStringLedger,
    allowed_structural_values: &mut BTreeSet<String>,
    groups: &mut BTreeMap<GroupKey, GroupEvidence>,
    fixtures: &mut Vec<Value>,
    diagnostics: &mut BTreeMap<String, u64>,
) {
    coverage.complete_records_considered = coverage.complete_records_considered.saturating_add(1);
    ledger.observe_value(&raw);
    let Some(evidence) = inspect_record(&raw, diagnostics) else {
        return;
    };

    if let Some(discriminator) = &evidence.discriminator_value {
        allowed_structural_values.insert(discriminator.clone());
    }
    let key = GroupKey {
        discriminator_path: evidence.discriminator_path.clone(),
        discriminator_value: evidence.discriminator_value.clone(),
    };
    groups.entry(key).or_default().merge(&evidence);

    if fixtures.len() >= MAX_TOTAL_FIXTURES {
        return;
    }
    let shape = FixtureShape {
        discriminator_path: evidence.discriminator_path,
        discriminator_value: evidence.discriminator_value,
        token_paths: evidence.token_values.keys().cloned().collect(),
        timestamp_path: evidence.timestamp_path,
        session_key_path: evidence.session_key_path,
        event_key_path: evidence.event_key_path,
    };
    match sanitize_fixture_record(&raw, &shape, fixtures.len()) {
        Ok(fixture) => fixtures.push(fixture),
        Err(PrivacyError::InvalidTokenCounter) => increment(diagnostics, "invalid_token_counter"),
        Err(_) => increment(diagnostics, "sanitization_error"),
    }
}

fn has_sufficient_evidence(groups: &BTreeMap<GroupKey, GroupEvidence>) -> bool {
    !groups.is_empty()
        && groups.values().all(|group| {
            !group.token_values.is_empty()
                && group.token_values.values().all(|values| values.len() >= 2)
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopReason {
    ByteLimit,
    RecordLimit,
    Io,
    SufficientEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordControl {
    Continue,
    StopSufficientEvidence,
}

struct FileInspectionResult {
    values: Vec<Value>,
    stop_reason: Option<StopReason>,
}

fn inspect_json_file(
    path: &Path,
    bytes_consumed: &mut u64,
    diagnostics: &mut BTreeMap<String, u64>,
    limits: ProbeLimits,
) -> FileInspectionResult {
    if limits.max_records == 0 {
        return FileInspectionResult {
            values: Vec::new(),
            stop_reason: Some(StopReason::RecordLimit),
        };
    }
    let Ok(file) = File::open(path) else {
        return FileInspectionResult {
            values: Vec::new(),
            stop_reason: Some(StopReason::Io),
        };
    };
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    let remaining_bytes = limits.max_bytes.saturating_sub(*bytes_consumed);
    if remaining_bytes == 0 {
        return FileInspectionResult {
            values: Vec::new(),
            stop_reason: Some(StopReason::ByteLimit),
        };
    }
    let record_limit = u64::try_from(limits.max_record_bytes).unwrap_or(u64::MAX);
    let read_limit = remaining_bytes.min(record_limit.saturating_add(1));
    let mut limited_reader = reader.by_ref().take(read_limit);
    if limited_reader.read_to_end(&mut bytes).is_err() {
        return FileInspectionResult {
            values: Vec::new(),
            stop_reason: Some(StopReason::Io),
        };
    }
    let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if bytes_consumed.saturating_add(length) > limits.max_bytes {
        return FileInspectionResult {
            values: Vec::new(),
            stop_reason: Some(StopReason::ByteLimit),
        };
    }
    *bytes_consumed = bytes_consumed.saturating_add(length);
    if bytes.len() > limits.max_record_bytes {
        increment(diagnostics, "record_too_large");
        return FileInspectionResult {
            values: Vec::new(),
            stop_reason: None,
        };
    }
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Array(values)) => FileInspectionResult {
            values,
            stop_reason: None,
        },
        Ok(value) => FileInspectionResult {
            values: vec![value],
            stop_reason: None,
        },
        Err(_) => {
            increment(diagnostics, "malformed_record");
            FileInspectionResult {
                values: Vec::new(),
                stop_reason: None,
            }
        }
    }
}

fn inspect_jsonl_file<F>(
    path: &Path,
    bytes_consumed: &mut u64,
    diagnostics: &mut BTreeMap<String, u64>,
    limits: ProbeLimits,
    mut consume_record: F,
) -> FileInspectionResult
where
    F: FnMut(Value, &mut BTreeMap<String, u64>) -> RecordControl,
{
    let Ok(file) = File::open(path) else {
        return FileInspectionResult {
            values: Vec::new(),
            stop_reason: Some(StopReason::Io),
        };
    };
    let mut reader = BufReader::new(file);
    let mut records_delivered = 0_u64;
    loop {
        if records_delivered >= limits.max_records {
            return FileInspectionResult {
                values: Vec::new(),
                stop_reason: Some(StopReason::RecordLimit),
            };
        }
        match read_bounded_jsonl_line(
            &mut reader,
            bytes_consumed,
            limits.max_bytes,
            limits.max_record_bytes,
        ) {
            BoundedLine::Eof => break,
            BoundedLine::ByteLimit => {
                return FileInspectionResult {
                    values: Vec::new(),
                    stop_reason: Some(StopReason::ByteLimit),
                };
            }
            BoundedLine::Io => {
                return FileInspectionResult {
                    values: Vec::new(),
                    stop_reason: Some(StopReason::Io),
                };
            }
            BoundedLine::Partial => {
                increment(diagnostics, "partial_final_line");
                break;
            }
            BoundedLine::Oversized => increment(diagnostics, "record_too_large"),
            BoundedLine::Complete(line) => match serde_json::from_slice::<Value>(&line) {
                Ok(value) => {
                    records_delivered = records_delivered.saturating_add(1);
                    if consume_record(value, diagnostics) == RecordControl::StopSufficientEvidence {
                        return FileInspectionResult {
                            values: Vec::new(),
                            stop_reason: Some(StopReason::SufficientEvidence),
                        };
                    }
                }
                Err(_) => increment(diagnostics, "malformed_record"),
            },
        }
    }
    FileInspectionResult {
        values: Vec::new(),
        stop_reason: None,
    }
}

enum BoundedLine {
    Eof,
    Complete(Vec<u8>),
    Partial,
    Oversized,
    ByteLimit,
    Io,
}

fn read_bounded_jsonl_line<R: BufRead>(
    reader: &mut R,
    bytes_consumed: &mut u64,
    max_bytes: u64,
    max_record_bytes: usize,
) -> BoundedLine {
    let mut line = Vec::new();
    let mut payload_bytes = 0_u64;
    loop {
        let remaining = max_bytes.saturating_sub(*bytes_consumed);
        if remaining == 0 {
            return BoundedLine::ByteLimit;
        }
        let buffer = match reader.fill_buf() {
            Ok(buffer) => buffer,
            Err(_) => return BoundedLine::Io,
        };
        if buffer.is_empty() {
            return if payload_bytes == 0 {
                BoundedLine::Eof
            } else if payload_bytes > max_record_bytes as u64 {
                BoundedLine::Oversized
            } else {
                BoundedLine::Partial
            };
        }

        let buffered = buffer.len();
        let available = buffered.min(usize::try_from(remaining).unwrap_or(usize::MAX));
        let slice = &buffer[..available];
        let newline = slice.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available, |index| index + 1);
        let payload = newline.map_or(consumed, |index| index);
        payload_bytes = payload_bytes.saturating_add(payload as u64);

        let retained_limit = max_record_bytes.saturating_add(1);
        let retained = retained_limit.saturating_sub(line.len()).min(payload);
        line.extend_from_slice(&slice[..retained]);
        reader.consume(consumed);
        *bytes_consumed = bytes_consumed.saturating_add(consumed as u64);

        if newline.is_some() {
            return if payload_bytes > max_record_bytes as u64 {
                BoundedLine::Oversized
            } else {
                BoundedLine::Complete(line)
            };
        }
        if consumed < buffered {
            return BoundedLine::ByteLimit;
        }
    }
}

fn inspect_record(raw: &Value, diagnostics: &mut BTreeMap<String, u64>) -> Option<RecordEvidence> {
    let mut evidence = RecordEvidence::default();
    walk_record(raw, "$", &mut evidence, 0, diagnostics);
    (!evidence.token_values.is_empty()).then_some(evidence)
}

fn walk_record(
    value: &Value,
    path: &str,
    evidence: &mut RecordEvidence,
    depth: usize,
    diagnostics: &mut BTreeMap<String, u64>,
) {
    if depth > 16 {
        return;
    }
    let Value::Object(fields) = value else {
        return;
    };
    for (key, value) in fields {
        let Some(path) = structural_path(path, key) else {
            continue;
        };
        let json_type = json_type(value).to_string();
        if is_discriminator_key(key) {
            if let Value::String(discriminator) = value {
                if is_valid_discriminator(discriminator) {
                    evidence.discriminator_path.get_or_insert(path.clone());
                    evidence
                        .discriminator_value
                        .get_or_insert_with(|| discriminator.clone());
                }
            }
            evidence.field_types.insert(path.clone(), json_type.clone());
        } else if is_timestamp_key(key) {
            if value.is_string() {
                evidence.timestamp_path.get_or_insert(path.clone());
            }
            evidence.field_types.insert(path.clone(), json_type.clone());
        } else if is_session_key(key) {
            evidence.session_key_path.get_or_insert(path.clone());
            evidence.field_types.insert(path.clone(), json_type.clone());
        } else if is_event_key(key) {
            evidence.event_key_path.get_or_insert(path.clone());
            evidence.field_types.insert(path.clone(), json_type.clone());
        }

        if is_token_key(key) {
            match value.as_u64() {
                Some(counter) => {
                    evidence.token_values.insert(path.clone(), counter);
                    evidence.field_types.insert(path.clone(), json_type);
                }
                None if value.is_number() => increment(diagnostics, "invalid_token_counter"),
                None => {}
            }
        }
        if value.is_object() {
            walk_record(value, &path, evidence, depth + 1, diagnostics);
        }
    }
}

fn structural_path(parent: &str, key: &str) -> Option<String> {
    if key.is_empty()
        || key.len() > 64
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return None;
    }
    Some(format!("{parent}.{key}"))
}

fn is_discriminator_key(key: &str) -> bool {
    matches!(key, "type" | "kind" | "event_type" | "eventType")
}

fn is_timestamp_key(key: &str) -> bool {
    matches!(key, "timestamp" | "ts" | "created_at" | "createdAt")
}

fn is_session_key(key: &str) -> bool {
    matches!(key, "session_id" | "sessionId")
}

fn is_event_key(key: &str) -> bool {
    matches!(key, "event_id" | "eventId" | "id" | "uuid")
}

fn is_token_key(key: &str) -> bool {
    key.ends_with("_tokens") || key.ends_with("Tokens")
}

fn is_valid_discriminator(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn increment(diagnostics: &mut BTreeMap<String, u64>, category: &str) {
    let entry = diagnostics.entry(category.to_string()).or_default();
    *entry = entry.saturating_add(1);
}

#[derive(Default)]
struct RecordEvidence {
    discriminator_path: Option<String>,
    discriminator_value: Option<String>,
    field_types: BTreeMap<String, String>,
    token_values: BTreeMap<String, u64>,
    timestamp_path: Option<String>,
    session_key_path: Option<String>,
    event_key_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    discriminator_path: Option<String>,
    discriminator_value: Option<String>,
}

#[derive(Default)]
struct GroupEvidence {
    key: Option<GroupKey>,
    field_types: BTreeMap<String, String>,
    token_paths: BTreeSet<String>,
    token_values: BTreeMap<String, Vec<u64>>,
    timestamp_path: Option<String>,
    session_key_path: Option<String>,
    event_key_path: Option<String>,
    count: u64,
}

impl GroupEvidence {
    fn merge(&mut self, evidence: &RecordEvidence) {
        if self.key.is_none() {
            self.key = Some(GroupKey {
                discriminator_path: evidence.discriminator_path.clone(),
                discriminator_value: evidence.discriminator_value.clone(),
            });
        }
        self.field_types.extend(
            evidence
                .field_types
                .iter()
                .map(|(path, kind)| (path.clone(), kind.clone())),
        );
        self.timestamp_path = self
            .timestamp_path
            .clone()
            .or_else(|| evidence.timestamp_path.clone());
        self.session_key_path = self
            .session_key_path
            .clone()
            .or_else(|| evidence.session_key_path.clone());
        self.event_key_path = self
            .event_key_path
            .clone()
            .or_else(|| evidence.event_key_path.clone());
        self.count = self.count.saturating_add(1);
        for (path, value) in &evidence.token_values {
            self.token_paths.insert(path.clone());
            self.token_values
                .entry(path.clone())
                .or_default()
                .push(*value);
        }
    }

    fn record_shape(&self) -> RecordShape {
        let key = self.key.as_ref();
        RecordShape {
            discriminator_path: key.and_then(|key| key.discriminator_path.clone()),
            discriminator_value: key.and_then(|key| key.discriminator_value.clone()),
            field_types: self
                .field_types
                .iter()
                .map(|(path, json_type)| FieldType {
                    path: path.clone(),
                    json_type: json_type.clone(),
                })
                .collect(),
            counter_paths: self.token_paths.iter().cloned().collect(),
            timestamp_path: self.timestamp_path.clone(),
            session_key_path: self.session_key_path.clone(),
            event_key_path: self.event_key_path.clone(),
            sampled_record_count: self.count,
        }
    }

    fn counter_sequences(&self) -> Vec<CounterSequence> {
        self.token_values
            .iter()
            .map(|(field_path, values)| {
                let behavior = classify(values);
                CounterSequence {
                    field_path: field_path.clone(),
                    observed_behavior: behavior,
                    synthetic_values: synthetic_values(behavior, values.len()),
                }
            })
            .collect()
    }
}

fn classify(values: &[u64]) -> ObservedBehavior {
    if values.len() < 2 {
        return ObservedBehavior::Uncertain;
    }
    let decreases = values.windows(2).filter(|pair| pair[1] < pair[0]).count();
    match decreases {
        0 => ObservedBehavior::Monotonic,
        1 => ObservedBehavior::ResetObserved,
        _ => ObservedBehavior::PerEvent,
    }
}

fn synthetic_values(behavior: ObservedBehavior, length: usize) -> Vec<u64> {
    let pattern: &[u64] = match behavior {
        ObservedBehavior::PerEvent => &[10, 20, 30, 40],
        ObservedBehavior::Monotonic => &[100, 150, 200, 250],
        ObservedBehavior::ResetObserved => &[100, 150, 20, 45],
        ObservedBehavior::Uncertain => &[10],
    };
    (0..length)
        .map(|index| pattern[index.min(pattern.len() - 1)])
        .collect()
}
