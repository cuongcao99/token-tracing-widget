# Native Provider Metadata Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and run a development-only Rust probe that characterizes native Windows Claude Code and Codex session formats and commits only validated synthetic fixtures and a sanitized compatibility report.

**Architecture:** A Cargo example discovers only the known provider roots beneath a supplied Windows profile, streams bounded JSON/JSONL records, retains structural and token metadata, and sanitizes every value before writing to ignored target output. Shared implementation precedes provider-specific live runs; Claude Code and Codex probes then run in parallel into separate ignored directories, followed by a single promotion and integration pass.

**Tech Stack:** Rust 2021, Serde/serde_json, SHA-256 string taint ledger, tempfile-based tests, Cargo examples, PowerShell, Tauri 2 workspace

**Spec:** `docs/superpowers/specs/2026-08-29-native-provider-probe-design.md`

## Global Constraints

- Keep this work on `feat/native-provider-probe`, created from the latest `dev` commit after this plan is committed.
- Probe native Windows Claude Code and Codex only; do not inspect WSL, invoke `wsl.exe`, or accept an arbitrary provider source root.
- Resolve only `%USERPROFILE%\.claude\projects` and `%USERPROFILE%\.codex\sessions`, or the same relative paths beneath a synthetic test profile.
- The probe is a Cargo example and must not be registered as a Tauri command or linked into the shipped executable.
- Never print, log, serialize, commit, or return raw session records, absolute source paths, identifiers, timestamps, prompts, responses, reasoning, tool payloads, credentials, repository contents, or working directories.
- Write live candidate output only below ignored `src-tauri/target/provider-probe/` until automated validation and agent review pass.
- Cap each provider run at five files, 50 MiB, 50,000 complete records, and 1 MiB per record.
- One provider's failure must not suppress the other provider's sanitized outcome.
- Add no network client, telemetry, sidecar, background service, frontend state library, CSS framework, or ORM.
- Preserve the existing frontend and Tauri bootstrap behavior.

## Execution ownership

- Root agent: Tasks 1, 4, and final independent verification.
- Shared Luna Max agent: Tasks 2 and 3 sequentially.
- Claude Luna Max agent and Codex Luna Max agent: Tasks 5 and 6 in parallel after Task 4 passes; they write only to separate ignored target directories and do not commit.
- Integration Luna Max agent: Task 7 after both provider agents finish.
- No agent may view or return raw session records. Provider agents consume only probe process status and validated files beneath `src-tauri/target/provider-probe/`.

---

### Task 1: Create the implementation branch

**Files:**
- Verify: `docs/superpowers/specs/2026-08-29-native-provider-probe-design.md`
- Verify: `docs/superpowers/plans/2026-08-29-native-provider-probe.md`

**Interfaces:**
- Consumes: latest local `dev` containing the approved design and this plan.
- Produces: checked-out `feat/native-provider-probe` pointing at the same commit as `dev`.

- [ ] **Step 1: Verify the planning branch and clean tracked tree**

Run:

```powershell
git branch --show-current
git status --short
git rev-parse dev
```

Expected: branch is `dev`; the only permitted untracked path is `.claude/`.

- [ ] **Step 2: Publish the approved planning baseline to dev**

Run:

```powershell
git push origin dev
```

Expected: `origin/dev` advances to the local design-and-plan commit; `origin/main` is unchanged.

- [ ] **Step 3: Create and check out the feature branch**

Run:

```powershell
git switch -c feat/native-provider-probe
```

Expected: `Switched to a new branch 'feat/native-provider-probe'`.

- [ ] **Step 4: Verify branch provenance**

Run:

```powershell
git branch --show-current
git merge-base --is-ancestor dev HEAD
git status --short --branch
```

Expected: current branch is `feat/native-provider-probe`; the ancestor command exits 0; `.claude/` remains untracked.

---

### Task 2: Define the report and fail-closed sanitizer

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/examples/provider_probe.rs`
- Create: `src-tauri/examples/provider_probe/report.rs`
- Create: `src-tauri/examples/provider_probe/sanitize.rs`

**Interfaces:**
- Consumes: raw `serde_json::Value` only inside process memory.
- Produces: `ProbeReport`, `ProviderReport`, `FixtureShape`, `SourceStringLedger`, `sanitize_fixture_record`, and `validate_serialized` for Task 3.

- [ ] **Step 1: Add the example test harness and failing privacy test**

Create `src-tauri/examples/provider_probe.rs`:

```rust
#[path = "provider_probe/report.rs"]
mod report;
#[path = "provider_probe/sanitize.rs"]
mod sanitize;

fn main() {}
```

Create `src-tauri/examples/provider_probe/sanitize.rs` with this test before defining the referenced types or functions:

```rust
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use serde_json::json;

    use super::{sanitize_fixture_record, validate_serialized, FixtureShape, SourceStringLedger};

    #[test]
    fn raw_strings_and_content_fields_never_reach_fixture_output() {
        let raw = json!({
            "type": "token_event",
            "session_id": "real-session-92af",
            "event_id": "real-event-b671",
            "timestamp": "2026-08-29T09:12:13Z",
            "usage": {"input_tokens": 1200, "output_tokens": 45},
            "message": {"content": "private prompt text"},
            "cwd": "C:\\Users\\person\\private-repository"
        });
        let mut ledger = SourceStringLedger::default();
        ledger.observe_value(&raw);
        let shape = FixtureShape {
            discriminator_path: Some("$.type".into()),
            discriminator_value: Some("token_event".into()),
            token_paths: vec!["$.usage.input_tokens".into(), "$.usage.output_tokens".into()],
            timestamp_path: Some("$.timestamp".into()),
            session_key_path: Some("$.session_id".into()),
            event_key_path: Some("$.event_id".into()),
        };

        let fixture = sanitize_fixture_record(&raw, &shape, 0).unwrap();
        let serialized = serde_json::to_string(&fixture).unwrap();
        validate_serialized(
            &serialized,
            &ledger,
            &BTreeSet::from(["token_event".to_string()]),
            Path::new(r"C:\Users\person"),
        )
        .unwrap();

        assert!(serialized.contains("session-synthetic-001"));
        assert!(serialized.contains("event-synthetic-001"));
        assert!(serialized.contains("2026-01-01T00:00:00Z"));
        assert!(!serialized.contains("real-session-92af"));
        assert!(!serialized.contains("real-event-b671"));
        assert!(!serialized.contains("private prompt text"));
        assert!(!serialized.contains("private-repository"));
        assert!(!serialized.contains("message"));
        assert!(!serialized.contains("cwd"));
    }
}
```

- [ ] **Step 2: Run the focused test and verify the missing-contract failure**

Run:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && "%USERPROFILE%\.cargo\bin\cargo.exe" test --manifest-path "src-tauri\Cargo.toml" --example provider_probe raw_strings_and_content_fields_never_reach_fixture_output'
```

Expected: FAIL because `FixtureShape`, `SourceStringLedger`, `sanitize_fixture_record`, and `validate_serialized` are not defined.

- [ ] **Step 3: Add test-only dependencies**

Append to `src-tauri/Cargo.toml`:

```toml
[dev-dependencies]
sha2 = "0.10"
tempfile = "3"
```

- [ ] **Step 4: Define the serialized report contract**

Create `src-tauri/examples/provider_probe/report.rs` with these exact public types:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeOutcome {
    Detected,
    NotDetected,
    PermissionDenied,
    UnsupportedFormat,
    LimitReached,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FieldType {
    pub path: String,
    pub json_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RecordShape {
    pub discriminator_path: Option<String>,
    pub discriminator_value: Option<String>,
    pub field_types: Vec<FieldType>,
    pub counter_paths: Vec<String>,
    pub timestamp_path: Option<String>,
    pub session_key_path: Option<String>,
    pub event_key_path: Option<String>,
    pub sampled_record_count: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservedBehavior {
    PerEvent,
    Monotonic,
    ResetObserved,
    Uncertain,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CounterSequence {
    pub field_path: String,
    pub observed_behavior: ObservedBehavior,
    pub synthetic_values: Vec<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Coverage {
    pub files_considered: u64,
    pub complete_records_considered: u64,
    pub byte_limit_reached: bool,
    pub record_limit_reached: bool,
    pub supported_shape_found: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DiagnosticCount {
    pub category: String,
    pub count: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProviderReport {
    pub provider: Provider,
    pub outcome: ProbeOutcome,
    pub layout_patterns: Vec<String>,
    pub record_shapes: Vec<RecordShape>,
    pub counter_sequences: Vec<CounterSequence>,
    pub diagnostic_counts: Vec<DiagnosticCount>,
    pub coverage: Coverage,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProbeReport {
    pub schema_version: u32,
    pub providers: Vec<ProviderReport>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FixtureManifest {
    pub schema_version: u32,
    pub provider: Provider,
    pub outcome: ProbeOutcome,
    pub layout_patterns: Vec<String>,
    pub record_shapes: Vec<RecordShape>,
    pub counter_sequences: Vec<CounterSequence>,
    pub fixture_record_count: u64,
}
```

- [ ] **Step 5: Implement fail-closed sanitization**

Implement these interfaces in `sanitize.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureShape {
    pub discriminator_path: Option<String>,
    pub discriminator_value: Option<String>,
    pub token_paths: Vec<String>,
    pub timestamp_path: Option<String>,
    pub session_key_path: Option<String>,
    pub event_key_path: Option<String>,
}

#[derive(Default)]
pub struct SourceStringLedger {
    hashes: std::collections::BTreeSet<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyError {
    InvalidJson,
    SourceStringLeak,
    AbsolutePath,
    Uri,
    OversizedStructuralString,
    InvalidTokenCounter,
}

impl SourceStringLedger {
    pub fn observe_value(&mut self, value: &serde_json::Value);
    pub fn contains(&self, value: &str) -> bool;
}

pub fn sanitize_fixture_record(
    raw: &serde_json::Value,
    shape: &FixtureShape,
    ordinal: usize,
) -> Result<serde_json::Value, PrivacyError>;

pub fn validate_serialized(
    serialized: &str,
    ledger: &SourceStringLedger,
    allowed_structural_values: &std::collections::BTreeSet<String>,
    profile_root: &std::path::Path,
) -> Result<(), PrivacyError>;
```

Implementation rules:

- `observe_value` hashes string values with SHA-256 and never hashes field names.
- `sanitize_fixture_record` constructs a new object; it never mutates or clones the raw object.
- Copy only the approved discriminator value. Write token fields as deterministic non-negative synthetic values beginning at 10 and increasing by 10 per ordinal.
- Write session/event identifiers as `session-synthetic-NNN` and `event-synthetic-NNN`.
- Write timestamps from `2026-01-01T00:00:00Z`, increasing seconds by ordinal.
- Add `"synthetic_unknown":{"ignored":true}` at the root.
- `validate_serialized` parses output JSON, checks every string value against the source ledger, permits only the explicit structural allow-list, and rejects drive paths, UNC paths, `file:`, `http:`, `https:`, and the supplied profile-root string.
- Define `PrivacyError` as an enum whose `Display` contains only a fixed category, never the rejected value.

- [ ] **Step 6: Run sanitizer tests**

Run the focused command from Step 2.

Expected: PASS with no raw value printed in test output.

- [ ] **Step 7: Add negative privacy tests**

Add tests that call `validate_serialized` with each of these serialized values and assert `Err` without formatting the rejected value:

```rust
r#"{"value":"C:\\Users\\person\\secret"}"#
r#"{"value":"\\\\server\\share\\secret"}"#
r#"{"value":"https://example.invalid/private"}"#
r#"{"value":"real-session-92af"}"#
```

Also assert a 65-character discriminator is rejected and a negative token counter passed to fixture construction is rejected before conversion to `u64`.

- [ ] **Step 8: Run all example tests and commit**

Run:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && "%USERPROFILE%\.cargo\bin\cargo.exe" test --manifest-path "src-tauri\Cargo.toml" --example provider_probe'
```

Expected: PASS.

Commit:

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/examples/provider_probe.rs src-tauri/examples/provider_probe/report.rs src-tauri/examples/provider_probe/sanitize.rs
git commit -m "test: define provider probe privacy boundary"
```

---

### Task 3: Implement bounded native discovery, inspection, and reporting

**Files:**
- Modify: `src-tauri/examples/provider_probe.rs`
- Modify: `src-tauri/examples/provider_probe/report.rs`
- Modify: `src-tauri/examples/provider_probe/sanitize.rs`
- Create: `src-tauri/examples/provider_probe/discovery.rs`
- Create: `src-tauri/examples/provider_probe/inspect.rs`

**Interfaces:**
- Consumes: Task 2 report/sanitizer types and known provider-relative roots.
- Produces: `ProbeArgs`, `ProbeLimits`, `discover_candidates`, `inspect_candidates`, `run_provider`, and validated candidate artifacts under `src-tauri/target/provider-probe/<run>`.

- [ ] **Step 1: Write native-root and bound tests**

Add to `discovery.rs` before implementation:

```rust
#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{discover_candidates, provider_root, ProbeLimits};
    use crate::report::Provider;

    #[test]
    fn roots_are_fixed_beneath_the_supplied_profile() {
        let profile = std::path::Path::new(r"C:\synthetic-profile");
        assert_eq!(provider_root(profile, Provider::Claude), profile.join(".claude").join("projects"));
        assert_eq!(provider_root(profile, Provider::Codex), profile.join(".codex").join("sessions"));
    }

    #[test]
    fn discovery_obeys_file_and_byte_limits() {
        let profile = tempdir().unwrap();
        let root = provider_root(profile.path(), Provider::Codex);
        fs::create_dir_all(&root).unwrap();
        for index in 0..7 {
            fs::write(root.join(format!("session-{index}.jsonl")), b"{}\n").unwrap();
        }
        let result = discover_candidates(
            profile.path(),
            Provider::Codex,
            ProbeLimits { max_files: 5, max_bytes: 10, max_records: 50_000, max_record_bytes: 1_048_576 },
        );
        assert_eq!(result.candidates.len(), 5);
        assert!(result.selected_bytes <= 10);
        assert!(result.candidates.iter().all(|candidate| !candidate.layout_pattern.contains("session-")));
    }
}
```

- [ ] **Step 2: Run discovery tests and verify missing implementations**

Run:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && "%USERPROFILE%\.cargo\bin\cargo.exe" test --manifest-path "src-tauri\Cargo.toml" --example provider_probe discovery::tests'
```

Expected: FAIL because the discovery interfaces do not exist.

- [ ] **Step 3: Implement fixed-root discovery**

Define:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootState {
    Readable,
    NotDetected,
    PermissionDenied,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct ProbeLimits {
    pub max_files: usize,
    pub max_bytes: u64,
    pub max_records: u64,
    pub max_record_bytes: usize,
}

impl Default for ProbeLimits {
    fn default() -> Self {
        Self { max_files: 5, max_bytes: 50 * 1024 * 1024, max_records: 50_000, max_record_bytes: 1024 * 1024 }
    }
}

pub struct CandidateFile {
    pub(crate) path: std::path::PathBuf,
    pub layout_pattern: String,
    pub size: u64,
}

pub struct DiscoveryResult {
    pub provider: crate::report::Provider,
    pub root_state: RootState,
    pub candidates: Vec<CandidateFile>,
    pub selected_bytes: u64,
}

pub fn provider_root(profile_root: &std::path::Path, provider: crate::report::Provider) -> std::path::PathBuf;
pub fn discover_candidates(profile_root: &std::path::Path, provider: crate::report::Provider, limits: ProbeLimits) -> DiscoveryResult;
```

Implementation rules:

- Recurse only below the fixed provider root.
- Consider regular `.json` and `.jsonl` files only.
- Sort by modification time descending, then sanitized layout pattern.
- Stop before adding a file that violates file or byte limits.
- Do not derive `Debug`, `Serialize`, or `Clone` for `CandidateFile`; the raw path must not cross the discovery/inspection boundary.
- Convert dynamic relative path segments to `<segment>`, numeric segments to `<number>`, and file stems to `<file>` while preserving extension and depth.
- Map `io::ErrorKind::NotFound` to `NotDetected` and `PermissionDenied` to `PermissionDenied`. Other enumeration failures become a bounded `discovery_error` diagnostic.

- [ ] **Step 4: Write streaming-inspection tests**

Create synthetic JSONL containing:

```json
{"type":"token_event","session_id":"private-a","event_id":"private-e1","timestamp":"2026-08-29T10:00:00Z","usage":{"input_tokens":100,"output_tokens":10},"message":{"content":"never emit this"}}
{"type":"token_event","session_id":"private-a","event_id":"private-e2","timestamp":"2026-08-29T10:00:01Z","usage":{"input_tokens":150,"output_tokens":25}}
{"type":"token_event"
```

Assert that `inspect_candidates` reports two complete records, one pending final line, token paths `$.usage.input_tokens` and `$.usage.output_tokens`, a monotonic input sequence, and sanitized fixtures that contain none of `private-a`, `private-e1`, or `never emit this`.

- [ ] **Step 5: Run inspection tests and verify the expected failure**

Run:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && "%USERPROFILE%\.cargo\bin\cargo.exe" test --manifest-path "src-tauri\Cargo.toml" --example provider_probe inspect::tests'
```

Expected: FAIL because `inspect_candidates` is missing.

- [ ] **Step 6: Implement streaming inspection and behavior classification**

Define:

```rust
pub struct InspectedProvider {
    pub report: crate::report::ProviderReport,
    pub manifest: crate::report::FixtureManifest,
    pub fixtures: Vec<serde_json::Value>,
    pub ledger: crate::sanitize::SourceStringLedger,
    pub allowed_structural_values: std::collections::BTreeSet<String>,
}

pub fn inspect_candidates(
    discovery: crate::discovery::DiscoveryResult,
    profile_root: &std::path::Path,
    limits: crate::discovery::ProbeLimits,
) -> InspectedProvider;
```

Inspection rules:

- Stream with `BufRead::read_until(b'\n', ...)`; reject a complete record larger than `max_record_bytes` with category `record_too_large`.
- Treat a non-newline-terminated final fragment as `partial_final_line` and never parse it.
- Parse complete lines as JSON values; count malformed complete lines as `malformed_record` without logging bytes or parser excerpts.
- Inventory only bounded structural paths: discriminator keys `type`, `kind`, `event_type`, `eventType`; timestamp keys `timestamp`, `ts`, `created_at`, `createdAt`; identity keys `session_id`, `sessionId`, `event_id`, `eventId`, `id`, `uuid`; and numeric leaf names ending in `_tokens` or `Tokens`.
- Accept discriminator values only when ASCII, at most 64 characters, and composed of letters, digits, `_`, `-`, or `.`.
- Group token-bearing records by discriminator and sorted relevant field paths.
- Classify numeric sequences: fewer than two values is `Uncertain`; no decreases is `Monotonic`; one decrease with all other pairs non-decreasing is `ResetObserved`; multiple decreases is `PerEvent`.
- Replace source numbers with `[10, 20, 30, 40]` for per-event evidence, `[100, 150, 200, 250]` for monotonic evidence, `[100, 150, 20, 45]` for reset evidence, or `[10]` for uncertain evidence, truncated to the observed length.
- Stop on the first reached global bound and mark coverage precisely.

- [ ] **Step 7: Implement validated artifact writing and CLI parsing**

Extend `provider_probe.rs` with the module paths and interfaces below:

```rust
#[path = "provider_probe/discovery.rs"]
mod discovery;
#[path = "provider_probe/inspect.rs"]
mod inspect;
#[path = "provider_probe/report.rs"]
mod report;
#[path = "provider_probe/sanitize.rs"]
mod sanitize;

#[derive(Debug)]
struct ProbeArgs {
    selection: ProviderSelection,
    profile_root: std::path::PathBuf,
    output_dir: std::path::PathBuf,
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

#[derive(Debug)]
enum ProbeError {
    Cli(CliError),
    Io(&'static str),
    Privacy(sanitize::PrivacyError),
    Serialization,
}

fn parse_args_from<I, S>(args: I) -> Result<ProbeArgs, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString>;

fn run_provider(
    profile_root: &std::path::Path,
    provider: report::Provider,
    limits: discovery::ProbeLimits,
) -> inspect::InspectedProvider;

fn write_validated_artifacts(
    output_dir: &std::path::Path,
    profile_root: &std::path::Path,
    providers: &[inspect::InspectedProvider],
) -> Result<(), ProbeError>;

fn render_compatibility_markdown(report: &report::ProbeReport) -> String;
```

CLI shape:

```text
provider_probe <claude|codex|all> --output <path-under-src-tauri-target-provider-probe> [--profile-root <synthetic-profile-root>]
```

Live runs default `--profile-root` to `USERPROFILE`. Reject live output outside `src-tauri/target/provider-probe`. Tests call `write_validated_artifacts` with a `tempfile` directory directly.

Write atomically after validation:

```text
<output>/probe-report.json
<output>/compatibility.md
<output>/claude/manifest.json
<output>/claude/records.jsonl
<output>/codex/manifest.json
<output>/codex/records.jsonl
```

For a single-provider run, omit the other provider directory. For `not_detected`, `permission_denied`, `unsupported_format`, or `limit_reached`, write the provider outcome to the report and manifest but omit `records.jsonl` unless validated fixture records exist.

`render_compatibility_markdown` must emit `# Native Windows Provider Formats`, followed by one `## Claude Code` or `## Codex` section per requested provider. Each section contains fixed `Outcome`, `Coverage`, `Layout patterns`, `Record shapes`, `Counter behavior`, `Identity and timestamp paths`, `Diagnostics`, and `Privacy validation` headings. Render report values as Markdown tables or `None observed`; never interpolate a raw path or source value.

Before renaming the temporary output directory into place, validate the report JSON, Markdown, every manifest, and every JSONL line. On privacy failure, remove only the newly created temporary directory and return a fixed-category error.

- [ ] **Step 8: Add CLI/output tests**

Tests must assert:

- `--output` outside `src-tauri/target/provider-probe` is rejected by CLI parsing for live runs.
- Synthetic-profile runs can write to a `tempfile` output through the direct function seam.
- `all` returns two independent `ProviderReport` values when one synthetic root is absent.
- Generated `probe-report.json`, manifests, JSONL, and `compatibility.md` parse and contain no source ledger values.
- Privacy failure leaves no final output directory.

- [ ] **Step 9: Run example and workspace gates**

Run:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && "%USERPROFILE%\.cargo\bin\cargo.exe" fmt --manifest-path "src-tauri\Cargo.toml" -- --check && "%USERPROFILE%\.cargo\bin\cargo.exe" test --manifest-path "src-tauri\Cargo.toml" --example provider_probe && "%USERPROFILE%\.cargo\bin\cargo.exe" test --manifest-path "src-tauri\Cargo.toml" && "%USERPROFILE%\.cargo\bin\cargo.exe" check --manifest-path "src-tauri\Cargo.toml"'
```

Expected: every command exits 0 with no raw source data in output.

- [ ] **Step 10: Commit the shared probe**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/examples/provider_probe.rs src-tauri/examples/provider_probe
git commit -m "feat: add native provider metadata probe"
```

---

### Task 4: Root privacy and shared-probe gate

**Files:**
- Review: `src-tauri/examples/provider_probe.rs`
- Review: `src-tauri/examples/provider_probe/*.rs`
- Review: `src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: committed shared probe from Tasks 2 and 3.
- Produces: approval to begin live provider runs; no source inspection occurs in this task.

- [ ] **Step 1: Review the diff for forbidden data paths**

Run:

```powershell
git diff dev...HEAD -- src-tauri/Cargo.toml src-tauri/examples
rg -n "println!|eprintln!|dbg!|Debug|canonicalize|read_to_string|USERPROFILE|message|content|prompt|reasoning|tool|credential|cwd|working_directory|repository" src-tauri/examples
```

Expected: output statements contain fixed categories and sanitized summaries only; `CandidateFile` cannot be debug-printed or serialized; content-bearing paths are omitted from fixtures.

- [ ] **Step 2: Run the shared gate independently**

Run the Task 3 Step 9 command.

Expected: all Rust gates pass.

- [ ] **Step 3: Run frontend regression checks**

Run:

```powershell
npm test -- --run
npm run build
```

Expected: two frontend tests pass and the production frontend build exits 0.

- [ ] **Step 4: Confirm runtime isolation**

Run:

```powershell
rg -n "provider_probe|examples::|mod probe" src-tauri/src src-tauri/tauri.conf.json src
```

Expected: no matches; the Tauri runtime does not reference the probe.

---

### Task 5: Probe native Claude Code safely

**Files:**
- Generate ignored: `src-tauri/target/provider-probe/claude/*`
- Do not modify tracked files.

**Interfaces:**
- Consumes: shared probe approved by Task 4; known native root `%USERPROFILE%\.claude\projects`.
- Produces: validated sanitized Claude candidate artifacts for Task 7.

- [ ] **Step 1: Run the Claude-only probe**

Run:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && set "PATH=%USERPROFILE%\.cargo\bin;%PATH%" && "%USERPROFILE%\.cargo\bin\cargo.exe" run --manifest-path "src-tauri\Cargo.toml" --example provider_probe -- claude --output "src-tauri\target\provider-probe\claude"'
```

Expected: exit 0 and a sanitized `probe-report.json`; no raw line, absolute source path, or identifier appears in terminal output.

- [ ] **Step 2: Validate only sanitized artifacts**

Inspect files under `src-tauri/target/provider-probe/claude` and confirm:

- `probe-report.json` parses.
- `compatibility.md` contains the Claude outcome and coverage.
- A `claude/manifest.json` exists.
- `claude/records.jsonl` exists only when validated fixtures were produced, and every complete line parses.
- No drive path, UNC path, URL, real timestamp, long identifier-like string, prompt text, response text, reasoning, tool payload, credential, working directory, or repository path appears.

Do not open or print any source file beneath `%USERPROFILE%\.claude\projects`.

- [ ] **Step 3: Report the sanitized outcome without committing**

Return only provider outcome, coverage counts, discovered structural paths, observed counter behaviors, candidate artifact paths beneath `target/provider-probe`, and validation result. Do not modify tracked files and do not commit.

---

### Task 6: Probe native Codex safely

**Files:**
- Generate ignored: `src-tauri/target/provider-probe/codex/*`
- Do not modify tracked files.

**Interfaces:**
- Consumes: shared probe approved by Task 4; known native root `%USERPROFILE%\.codex\sessions`.
- Produces: validated sanitized Codex candidate artifacts for Task 7.

- [ ] **Step 1: Run the Codex-only probe**

Run:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && set "PATH=%USERPROFILE%\.cargo\bin;%PATH%" && "%USERPROFILE%\.cargo\bin\cargo.exe" run --manifest-path "src-tauri\Cargo.toml" --example provider_probe -- codex --output "src-tauri\target\provider-probe\codex"'
```

Expected: exit 0 and a sanitized `probe-report.json`; no raw line, absolute source path, or identifier appears in terminal output.

- [ ] **Step 2: Validate only sanitized artifacts**

Inspect files under `src-tauri/target/provider-probe/codex` and confirm:

- `probe-report.json` parses.
- `compatibility.md` contains the Codex outcome and coverage.
- A `codex/manifest.json` exists.
- `codex/records.jsonl` exists only when validated fixtures were produced, and every complete line parses.
- No drive path, UNC path, URL, real timestamp, long identifier-like string, prompt text, response text, reasoning, tool payload, credential, working directory, or repository path appears.

Do not open or print any source file beneath `%USERPROFILE%\.codex\sessions`.

- [ ] **Step 3: Report the sanitized outcome without committing**

Return only provider outcome, coverage counts, discovered structural paths, observed counter behaviors, candidate artifact paths beneath `target/provider-probe`, and validation result. Do not modify tracked files and do not commit.

---

### Task 7: Integrate sanitized fixtures and compatibility report

**Files:**
- Create when a Claude outcome manifest is generated: `src-tauri/tests/fixtures/providers/claude/native_windows/manifest.json`
- Create when Claude has validated records: `src-tauri/tests/fixtures/providers/claude/native_windows/records.jsonl`
- Create when a Codex outcome manifest is generated: `src-tauri/tests/fixtures/providers/codex/native_windows/manifest.json`
- Create when Codex has validated records: `src-tauri/tests/fixtures/providers/codex/native_windows/records.jsonl`
- Create: `docs/compatibility/2026-08-29-native-provider-formats.md`
- Generate ignored: `src-tauri/target/provider-probe/all/*`

**Interfaces:**
- Consumes: Tasks 5 and 6 sanitized outcomes plus the shared probe.
- Produces: committed sanitized compatibility evidence for future adapter plans.

- [ ] **Step 1: Run a combined probe for one coherent report**

Run:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && set "PATH=%USERPROFILE%\.cargo\bin;%PATH%" && "%USERPROFILE%\.cargo\bin\cargo.exe" run --manifest-path "src-tauri\Cargo.toml" --example provider_probe -- all --output "src-tauri\target\provider-probe\all"'
```

Expected: exit 0; `probe-report.json` contains exactly one independent Claude report and one independent Codex report.

- [ ] **Step 2: Compare combined output with provider runs**

Compare sanitized outcomes, structural paths, counter behaviors, and coverage with Tasks 5 and 6. If a provider outcome or shape differs because local files changed, use the newer combined output and record the changed coverage in `compatibility.md`; never inspect raw source data to reconcile it.

- [ ] **Step 3: Run final privacy review before promotion**

Review `src-tauri/target/provider-probe/all` recursively. Reject promotion if any file contains a drive path, UNC path, URL, real timestamp, source identifier, conversational text, reasoning, tool payload, credential, working directory, repository path, or value outside the structural allow-list.

Expected: all generated files contain schema metadata and synthetic values only.

- [ ] **Step 4: Promote generated artifacts mechanically**

Create only the destination directories corresponding to provider artifacts that exist, then copy validated files:

```powershell
New-Item -ItemType Directory -Force 'docs\compatibility' | Out-Null
Copy-Item -LiteralPath 'src-tauri\target\provider-probe\all\compatibility.md' -Destination 'docs\compatibility\2026-08-29-native-provider-formats.md'

$probeProviders = @('claude', 'codex')
foreach ($probeProvider in $probeProviders) {
    $probeSource = Join-Path 'src-tauri\target\provider-probe\all' $probeProvider
    $probeManifest = Join-Path $probeSource 'manifest.json'
    if (-not (Test-Path -LiteralPath $probeManifest)) {
        continue
    }

    $probeDestination = Join-Path 'src-tauri\tests\fixtures\providers' "$probeProvider\native_windows"
    New-Item -ItemType Directory -Force $probeDestination | Out-Null
    Copy-Item -LiteralPath $probeManifest -Destination (Join-Path $probeDestination 'manifest.json')

    $probeRecords = Join-Path $probeSource 'records.jsonl'
    if (Test-Path -LiteralPath $probeRecords) {
        Copy-Item -LiteralPath $probeRecords -Destination (Join-Path $probeDestination 'records.jsonl')
    }
}
```

The guarded loop creates a fixture directory only when the combined output produced a manifest and copies `records.jsonl` only when validated fixture records exist. Do not create fabricated records for absent, blocked, unsupported, or insufficiently sampled providers.

- [ ] **Step 5: Add fixture parsing and privacy regression tests**

Add tests to `report.rs` that load every committed `manifest.json` and every complete line in committed `records.jsonl` through paths relative to `env!("CARGO_MANIFEST_DIR")`. Reuse `validate_serialized` with an empty source ledger plus forbidden path/content checks, and assert each manifest's provider matches its directory.

- [ ] **Step 6: Run focused and full gates**

Run:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && "%USERPROFILE%\.cargo\bin\cargo.exe" fmt --manifest-path "src-tauri\Cargo.toml" -- --check && "%USERPROFILE%\.cargo\bin\cargo.exe" test --manifest-path "src-tauri\Cargo.toml" --example provider_probe && "%USERPROFILE%\.cargo\bin\cargo.exe" test --manifest-path "src-tauri\Cargo.toml" && "%USERPROFILE%\.cargo\bin\cargo.exe" check --manifest-path "src-tauri\Cargo.toml"'
npm test -- --run
npm run build
```

Expected: all Rust and frontend commands exit 0.

- [ ] **Step 7: Commit sanitized compatibility evidence**

Stage only tracked probe code changes, fixture directories that exist, and the compatibility report. Confirm `src-tauri/target`, `.claude/`, and provider source directories are absent from the index.

```powershell
git add src-tauri/examples src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tests/fixtures docs/compatibility/2026-08-29-native-provider-formats.md
git diff --cached --check
git diff --cached --name-only
git commit -m "test: add native provider compatibility fixtures"
```

---

### Task 8: Final root verification and feature-branch publication

**Files:**
- Review: all changes in `dev...feat/native-provider-probe`
- Verify: `docs/compatibility/2026-08-29-native-provider-formats.md`
- Verify: `src-tauri/tests/fixtures/providers/**`

**Interfaces:**
- Consumes: completed feature branch from Task 7.
- Produces: verified remote feature branch ready for review; `dev` and `main` remain unchanged.

- [ ] **Step 1: Review complete branch scope**

Run:

```powershell
git diff --stat dev...HEAD
git diff --check dev...HEAD
git status --short --branch
```

Expected: only probe tooling, Cargo lock/dependency changes, sanitized fixtures, and compatibility documentation differ; `.claude/` is the only untracked path.

- [ ] **Step 2: Re-run every completion gate**

Run Task 7 Step 6 exactly, then run the integrated Tauri build:

```powershell
cmd.exe /d /s /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && set "PATH=%USERPROFILE%\.cargo\bin;%PATH%" && npm run tauri build -- --debug --no-bundle'
```

Expected: all tests and checks pass; the integrated executable builds without warnings attributable to this feature.

- [ ] **Step 3: Confirm remote protected branches are unchanged**

Run:

```powershell
git rev-parse dev
git merge-base dev HEAD
git ls-remote --heads origin dev main
```

Expected: local `dev`, `origin/dev`, and the merge base match at the approved design-and-plan commit; `origin/main` remains at its finalized bootstrap commit; feature commits exist only on `feat/native-provider-probe`.

- [ ] **Step 4: Push the feature branch**

```powershell
git push -u origin feat/native-provider-probe
```

Expected: the remote feature branch is created; `main` and `dev` are not updated.
