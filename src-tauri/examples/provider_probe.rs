#[path = "provider_probe/discovery.rs"]
mod discovery;
#[path = "provider_probe/inspect.rs"]
mod inspect;
#[path = "provider_probe/report.rs"]
mod report;
#[path = "provider_probe/sanitize.rs"]
mod sanitize;

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct ProbeArgs {
    selection: ProviderSelection,
    profile_root: PathBuf,
    output_dir: PathBuf,
}

#[derive(Debug, Clone, Copy)]
enum ProviderSelection {
    Claude,
    Codex,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliError {
    Usage,
    MissingProfile,
    OutputOutsideTarget,
    InvalidSelection,
}

#[allow(dead_code)]
#[derive(Debug)]
enum ProbeError {
    Cli(CliError),
    Io(&'static str),
    Privacy(sanitize::PrivacyError),
    Serialization,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let category = match self {
            Self::Usage => "usage",
            Self::MissingProfile => "missing_profile",
            Self::OutputOutsideTarget => "output_outside_target",
            Self::InvalidSelection => "invalid_selection",
        };
        formatter.write_str(category)
    }
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cli(error) => error.fmt(formatter),
            Self::Io(category) => formatter.write_str(category),
            Self::Privacy(error) => error.fmt(formatter),
            Self::Serialization => formatter.write_str("serialization"),
        }
    }
}

fn main() {
    let args = match parse_args_from(std::env::args_os()) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("provider_probe_error:{error}");
            std::process::exit(2);
        }
    };
    let providers = match args.selection {
        ProviderSelection::Claude => vec![report::Provider::Claude],
        ProviderSelection::Codex => vec![report::Provider::Codex],
        ProviderSelection::All => vec![report::Provider::Claude, report::Provider::Codex],
    };
    let inspected = providers
        .into_iter()
        .map(|provider| {
            run_provider(
                &args.profile_root,
                provider,
                discovery::ProbeLimits::default(),
            )
        })
        .collect::<Vec<_>>();
    if let Err(error) = write_validated_artifacts(&args.output_dir, &args.profile_root, &inspected)
    {
        eprintln!("provider_probe_error:{error}");
        std::process::exit(1);
    }
}

fn parse_args_from<I, S>(args: I) -> Result<ProbeArgs, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut args = args
        .into_iter()
        .map(|arg| arg.into())
        .collect::<Vec<_>>()
        .into_iter();
    let _program = args.next();
    let selection = match args
        .next()
        .map(|arg| arg.to_string_lossy().into_owned())
        .as_deref()
    {
        Some("claude") => ProviderSelection::Claude,
        Some("codex") => ProviderSelection::Codex,
        Some("all") => ProviderSelection::All,
        Some(_) => return Err(CliError::InvalidSelection),
        None => return Err(CliError::Usage),
    };

    let mut output_dir = None;
    let mut profile_root = None;
    while let Some(argument) = args.next() {
        match argument.to_string_lossy().as_ref() {
            "--output" => {
                let Some(value) = args.next() else {
                    return Err(CliError::Usage);
                };
                output_dir = Some(PathBuf::from(value));
            }
            "--profile-root" => {
                let Some(value) = args.next() else {
                    return Err(CliError::Usage);
                };
                profile_root = Some(PathBuf::from(value));
            }
            _ => return Err(CliError::Usage),
        }
    }

    let output_dir = output_dir.ok_or(CliError::Usage)?;
    let output_dir = resolve_path(&output_dir);
    if !is_under_provider_probe_target(&output_dir) {
        return Err(CliError::OutputOutsideTarget);
    }
    let profile_root = profile_root
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or(CliError::MissingProfile)?;
    Ok(ProbeArgs {
        selection,
        profile_root,
        output_dir,
    })
}

fn resolve_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn is_under_provider_probe_target(path: &Path) -> bool {
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return false;
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("provider-probe");
    let normalize = |value: &Path| {
        value
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_ascii_lowercase()
    };
    let path = normalize(path);
    let target = normalize(&target);
    path == target || path.starts_with(&(target + "/"))
}

fn run_provider(
    profile_root: &Path,
    provider: report::Provider,
    limits: discovery::ProbeLimits,
) -> inspect::InspectedProvider {
    let discovery = discovery::discover_candidates(profile_root, provider, limits);
    inspect::inspect_candidates(discovery, profile_root, limits)
}

fn write_validated_artifacts(
    output_dir: &Path,
    profile_root: &Path,
    providers: &[inspect::InspectedProvider],
) -> Result<(), ProbeError> {
    let parent = output_dir.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|_| ProbeError::Io("output_parent"))?;
    let temporary = tempfile::tempdir_in(parent).map_err(|_| ProbeError::Io("output_temp"))?;
    let temporary_path = temporary.path().to_path_buf();

    let report = report::ProbeReport {
        schema_version: 1,
        providers: providers
            .iter()
            .map(|provider| provider.report.clone())
            .collect(),
    };
    let report_json =
        serde_json::to_string_pretty(&report).map_err(|_| ProbeError::Serialization)?;
    let report_allowed = report_allowed_values(&report);
    sanitize::validate_serialized(
        &report_json,
        &sanitize::SourceStringLedger::default(),
        &report_allowed,
        profile_root,
    )
    .map_err(ProbeError::Privacy)?;
    let compatibility = render_compatibility_markdown(&report);
    validate_markdown(&compatibility).map_err(ProbeError::Privacy)?;

    fs::write(temporary_path.join("probe-report.json"), report_json)
        .map_err(|_| ProbeError::Io("report_write"))?;
    fs::write(temporary_path.join("compatibility.md"), compatibility)
        .map_err(|_| ProbeError::Io("compatibility_write"))?;

    for provider in providers {
        let provider_name = provider.report.provider.as_str();
        let provider_dir = temporary_path.join(provider_name);
        fs::create_dir_all(&provider_dir).map_err(|_| ProbeError::Io("provider_directory"))?;
        let manifest_json = serde_json::to_string_pretty(&provider.manifest)
            .map_err(|_| ProbeError::Serialization)?;
        let mut manifest_allowed = report_allowed.clone();
        manifest_allowed.extend(provider.allowed_structural_values.iter().cloned());
        sanitize::validate_serialized(
            &manifest_json,
            &provider.ledger,
            &manifest_allowed,
            profile_root,
        )
        .map_err(ProbeError::Privacy)?;
        fs::write(provider_dir.join("manifest.json"), manifest_json)
            .map_err(|_| ProbeError::Io("manifest_write"))?;

        if !provider.fixtures.is_empty() {
            let mut records = String::new();
            for fixture in &provider.fixtures {
                let serialized =
                    serde_json::to_string(fixture).map_err(|_| ProbeError::Serialization)?;
                sanitize::validate_serialized(
                    &serialized,
                    &provider.ledger,
                    &provider.allowed_structural_values,
                    profile_root,
                )
                .map_err(ProbeError::Privacy)?;
                records.push_str(&serialized);
                records.push('\n');
            }
            fs::write(provider_dir.join("records.jsonl"), records)
                .map_err(|_| ProbeError::Io("records_write"))?;
        }
    }

    if output_dir.exists() {
        fs::remove_dir_all(output_dir).map_err(|_| ProbeError::Io("output_replace"))?;
    }
    let temporary_path = temporary.keep();
    if let Err(_) = fs::rename(&temporary_path, output_dir) {
        let _ = fs::remove_dir_all(&temporary_path);
        return Err(ProbeError::Io("output_rename"));
    }
    Ok(())
}

fn report_allowed_values(report: &report::ProbeReport) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    for provider in &report.providers {
        values.insert(provider.provider.as_str().to_string());
        values.insert(outcome_name(provider.outcome).to_string());
        values.extend(provider.layout_patterns.iter().cloned());
        for shape in &provider.record_shapes {
            add_optional(&mut values, shape.discriminator_path.as_ref());
            add_optional(&mut values, shape.discriminator_value.as_ref());
            for field in &shape.field_types {
                values.insert(field.path.clone());
                values.insert(field.json_type.clone());
            }
            values.extend(shape.counter_paths.iter().cloned());
            add_optional(&mut values, shape.timestamp_path.as_ref());
            add_optional(&mut values, shape.session_key_path.as_ref());
            add_optional(&mut values, shape.event_key_path.as_ref());
        }
        for sequence in &provider.counter_sequences {
            values.insert(sequence.field_path.clone());
            values.insert(behavior_name(sequence.observed_behavior).to_string());
        }
        values.extend(
            provider
                .diagnostic_counts
                .iter()
                .map(|diagnostic| diagnostic.category.clone()),
        );
    }
    values
}

fn add_optional(values: &mut BTreeSet<String>, value: Option<&String>) {
    if let Some(value) = value {
        values.insert(value.clone());
    }
}

fn outcome_name(outcome: report::ProbeOutcome) -> &'static str {
    match outcome {
        report::ProbeOutcome::Detected => "detected",
        report::ProbeOutcome::NotDetected => "not_detected",
        report::ProbeOutcome::PermissionDenied => "permission_denied",
        report::ProbeOutcome::UnsupportedFormat => "unsupported_format",
        report::ProbeOutcome::LimitReached => "limit_reached",
    }
}

fn behavior_name(behavior: report::ObservedBehavior) -> &'static str {
    match behavior {
        report::ObservedBehavior::PerEvent => "per_event",
        report::ObservedBehavior::Monotonic => "monotonic",
        report::ObservedBehavior::ResetObserved => "reset_observed",
        report::ObservedBehavior::Uncertain => "uncertain",
    }
}

fn render_compatibility_markdown(report: &report::ProbeReport) -> String {
    let mut markdown = String::from("# Native Windows Provider Formats\n\n");
    for provider in &report.providers {
        let title = match provider.provider {
            report::Provider::Claude => "Claude Code",
            report::Provider::Codex => "Codex",
        };
        markdown.push_str(&format!("## {title}\n\n"));
        markdown.push_str("### Outcome\n\n");
        markdown.push_str(&format!(
            "| Field | Value |\n| --- | --- |\n| Outcome | {} |\n\n",
            outcome_name(provider.outcome)
        ));
        markdown.push_str("### Coverage\n\n");
        markdown.push_str("| Files | Complete records | Byte limit | Record limit | Supported shape |\n| ---: | ---: | --- | --- | --- |\n");
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n\n",
            provider.coverage.files_considered,
            provider.coverage.complete_records_considered,
            provider.coverage.byte_limit_reached,
            provider.coverage.record_limit_reached,
            provider.coverage.supported_shape_found,
        ));
        markdown.push_str("### Layout patterns\n\n");
        render_lines(&mut markdown, &provider.layout_patterns);
        markdown.push_str("### Record shapes\n\n");
        if provider.record_shapes.is_empty() {
            markdown.push_str("None observed\n\n");
        } else {
            markdown
                .push_str("| Discriminator | Counters | Sampled records |\n| --- | --- | ---: |\n");
            for shape in &provider.record_shapes {
                markdown.push_str(&format!(
                    "| {} | {} | {} |\n",
                    optional_text(shape.discriminator_value.as_ref()),
                    if shape.counter_paths.is_empty() {
                        "None observed".to_string()
                    } else {
                        shape.counter_paths.join(", ")
                    },
                    shape.sampled_record_count,
                ));
            }
            markdown.push('\n');
        }
        markdown.push_str("### Counter behavior\n\n");
        if provider.counter_sequences.is_empty() {
            markdown.push_str("None observed\n\n");
        } else {
            markdown.push_str("| Field | Behavior | Synthetic sequence |\n| --- | --- | --- |\n");
            for sequence in &provider.counter_sequences {
                markdown.push_str(&format!(
                    "| {} | {} | {} |\n",
                    sequence.field_path,
                    behavior_name(sequence.observed_behavior),
                    sequence
                        .synthetic_values
                        .iter()
                        .map(u64::to_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                ));
            }
            markdown.push('\n');
        }
        markdown.push_str("### Identity and timestamp paths\n\n");
        if provider.record_shapes.is_empty() {
            markdown.push_str("None observed\n\n");
        } else {
            for shape in &provider.record_shapes {
                markdown.push_str(&format!(
                    "- Session: {}; Event: {}; Timestamp: {}\n",
                    optional_text(shape.session_key_path.as_ref()),
                    optional_text(shape.event_key_path.as_ref()),
                    optional_text(shape.timestamp_path.as_ref()),
                ));
            }
            markdown.push('\n');
        }
        markdown.push_str("### Diagnostics\n\n");
        if provider.diagnostic_counts.is_empty() {
            markdown.push_str("None observed\n\n");
        } else {
            for diagnostic in &provider.diagnostic_counts {
                markdown.push_str(&format!(
                    "- {}: {}\n",
                    diagnostic.category, diagnostic.count
                ));
            }
            markdown.push('\n');
        }
        markdown.push_str("### Artifacts\n\n");
        markdown.push_str(&format!(
            "- Manifest: {}/manifest.json\n",
            provider.provider.as_str()
        ));
        if provider.coverage.supported_shape_found {
            markdown.push_str(&format!(
                "- Records: {}/records.jsonl\n",
                provider.provider.as_str()
            ));
        }
        markdown.push('\n');
        markdown.push_str("### Privacy validation\n\nValidated before write.\n\n");
    }
    markdown
}

fn render_lines(markdown: &mut String, lines: &[String]) {
    if lines.is_empty() {
        markdown.push_str("None observed\n\n");
    } else {
        for line in lines {
            markdown.push_str(&format!("- {line}\n"));
        }
        markdown.push('\n');
    }
}

fn optional_text(value: Option<&String>) -> String {
    value
        .cloned()
        .unwrap_or_else(|| "None observed".to_string())
}

fn validate_markdown(markdown: &str) -> Result<(), sanitize::PrivacyError> {
    if markdown.contains("\\\\") || markdown.contains("//") {
        return Err(sanitize::PrivacyError::AbsolutePath);
    }
    if markdown.contains("http:") || markdown.contains("https:") || markdown.contains("file:") {
        return Err(sanitize::PrivacyError::Uri);
    }
    for value in markdown.split_whitespace() {
        if value.len() > 64 {
            return Err(sanitize::PrivacyError::OversizedStructuralString);
        }
    }
    Ok(())
}

#[cfg(test)]
mod cli_tests {
    use std::fs;
    use tempfile::tempdir;

    use super::discovery::{provider_root, ProbeLimits};
    use super::inspect::InspectedProvider;
    use super::report::{
        Coverage, FixtureManifest, ProbeOutcome, ProbeReport, Provider, ProviderReport,
    };
    use super::sanitize::SourceStringLedger;
    use super::{parse_args_from, run_provider, write_validated_artifacts, CliError};

    #[test]
    fn live_output_must_be_under_provider_probe_target() {
        let outside = tempdir().unwrap();
        let result = parse_args_from([
            "provider_probe",
            "claude",
            "--output",
            outside.path().to_str().unwrap(),
        ]);

        assert!(matches!(result, Err(CliError::OutputOutsideTarget)));
    }

    #[test]
    fn synthetic_profile_can_write_through_direct_seam() {
        let profile = tempdir().unwrap();
        let root = provider_root(profile.path(), Provider::Claude);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("session.jsonl"),
            concat!(
                r#"{"type":"token_event","session_id":"synthetic-session","event_id":"synthetic-event","timestamp":"2026-08-29T10:00:00Z","usage":{"input_tokens":100,"output_tokens":10}}"#,
                "\n",
            ),
        )
        .unwrap();

        let inspected = run_provider(profile.path(), Provider::Claude, ProbeLimits::default());
        let output = profile.path().join("candidate-output");
        write_validated_artifacts(&output, profile.path(), &[inspected]).unwrap();

        let report: ProbeReport =
            serde_json::from_str(&fs::read_to_string(output.join("probe-report.json")).unwrap())
                .unwrap();
        assert_eq!(report.providers.len(), 1);
        assert!(output.join("claude/manifest.json").is_file());
        assert!(output.join("claude/records.jsonl").is_file());
        assert!(output.join("compatibility.md").is_file());
        let compatibility = fs::read_to_string(output.join("compatibility.md")).unwrap();
        assert!(compatibility.contains("Manifest: claude/manifest.json"));
        assert!(compatibility.contains("Records: claude/records.jsonl"));
        assert!(!compatibility.contains(profile.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn all_keeps_absent_provider_independent() {
        let profile = tempdir().unwrap();
        let root = provider_root(profile.path(), Provider::Codex);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("session.jsonl"),
            concat!(
                r#"{"type":"token_event","usage":{"input_tokens":100}}"#,
                "\n"
            ),
        )
        .unwrap();

        let providers = [
            run_provider(profile.path(), Provider::Claude, ProbeLimits::default()),
            run_provider(profile.path(), Provider::Codex, ProbeLimits::default()),
        ];
        let reports = providers
            .iter()
            .map(|provider| provider.report.clone())
            .collect::<Vec<_>>();

        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].outcome, ProbeOutcome::NotDetected);
        assert_eq!(reports[1].outcome, ProbeOutcome::Detected);
    }

    #[test]
    fn compatibility_uses_fixed_provider_relative_artifact_paths() {
        let report = ProbeReport {
            schema_version: 1,
            providers: vec![
                ProviderReport {
                    provider: Provider::Claude,
                    outcome: ProbeOutcome::Detected,
                    layout_patterns: Vec::new(),
                    record_shapes: Vec::new(),
                    counter_sequences: Vec::new(),
                    diagnostic_counts: Vec::new(),
                    coverage: Coverage {
                        supported_shape_found: true,
                        ..Coverage::default()
                    },
                },
                ProviderReport {
                    provider: Provider::Codex,
                    outcome: ProbeOutcome::NotDetected,
                    layout_patterns: Vec::new(),
                    record_shapes: Vec::new(),
                    counter_sequences: Vec::new(),
                    diagnostic_counts: Vec::new(),
                    coverage: Coverage::default(),
                },
            ],
        };

        let compatibility = super::render_compatibility_markdown(&report);

        assert!(compatibility.contains("Manifest: claude/manifest.json"));
        assert!(compatibility.contains("Records: claude/records.jsonl"));
        assert!(compatibility.contains("Manifest: codex/manifest.json"));
        assert!(!compatibility.contains("Records: codex/records.jsonl"));
    }

    #[test]
    fn privacy_failure_leaves_no_final_output_directory() {
        let output_parent = tempdir().unwrap();
        let output = output_parent.path().join("candidate-output");
        let report = ProviderReport {
            provider: Provider::Claude,
            outcome: ProbeOutcome::Detected,
            layout_patterns: Vec::new(),
            record_shapes: Vec::new(),
            counter_sequences: Vec::new(),
            diagnostic_counts: Vec::new(),
            coverage: Coverage {
                supported_shape_found: true,
                ..Coverage::default()
            },
        };
        let inspected = InspectedProvider {
            manifest: FixtureManifest {
                schema_version: 1,
                provider: Provider::Claude,
                outcome: ProbeOutcome::Detected,
                layout_patterns: Vec::new(),
                record_shapes: Vec::new(),
                counter_sequences: Vec::new(),
                fixture_record_count: 1,
            },
            report,
            fixtures: vec![serde_json::json!({"value":"C:\\Users\\person\\secret"})],
            ledger: SourceStringLedger::default(),
            allowed_structural_values: Default::default(),
        };

        assert!(write_validated_artifacts(&output, output_parent.path(), &[inspected]).is_err());
        assert!(!output.exists());
    }
}
