# Native Provider Metadata Probe Design

**Date:** 2026-08-29
**Status:** Approved design
**Parent design:** `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`

## Purpose

Establish the native Windows session layouts and token-record shapes produced by the installed Claude Code and Codex versions before implementing either provider adapter. The probe must produce repeatable compatibility evidence and synthetic fixtures without writing raw session content, local identifiers, absolute paths, or real timestamps anywhere.

## Scope

This slice covers native Windows Claude Code and Codex sources under their known per-user defaults. It does not inspect WSL, invoke `wsl.exe`, accept arbitrary source roots, implement runtime provider adapters, change the Tauri command surface, or add collection and persistence behavior.

The deliverables are:

- A development-only Rust probe with automated privacy checks.
- A sanitized compatibility report for both native providers.
- One manifest and one synthetic JSONL fixture set per detected provider.
- Explicit independent outcomes when a provider is absent, blocked, unsupported, or insufficiently sampled.

## Approach

Use a Cargo example rather than manual shell inspection or adapter-first development. The example is opt-in development tooling and is not linked into the shipped Tauri executable. It can be tested with synthetic temporary directories and run locally against known provider defaults while keeping the production runtime unchanged.

Manual inspection was rejected because it is difficult to reproduce and makes accidental disclosure easier. Adapter-first development was rejected because it would mix format discovery with production parsing and encode assumptions before the installed formats are understood.

## Structure

```text
src-tauri/examples/provider_probe.rs
src-tauri/examples/provider_probe/
  discovery.rs
  inspect.rs
  sanitize.rs
  report.rs
docs/compatibility/2026-08-29-native-provider-formats.md
src-tauri/tests/fixtures/providers/claude/native_windows/
  manifest.json
  records.jsonl
src-tauri/tests/fixtures/providers/codex/native_windows/
  manifest.json
  records.jsonl
```

The executable accepts `claude`, `codex`, or `all`. Live runs resolve provider roots from the current Windows user profile. Tests may supply a synthetic profile root, but the probe still resolves only the provider-specific relative defaults beneath that root.

All live output is written below the ignored `src-tauri/target/provider-probe/` directory. Nothing is copied into committed fixture or documentation paths until automated validation passes and an agent reviews the sanitized artifacts.

## Probe model

The probe emits a versioned `ProbeReport` containing one `ProviderReport` per requested provider.

```text
ProbeReport
  schema_version
  providers[]

ProviderReport
  provider
  outcome
  layout_patterns[]
  record_shapes[]
  counter_sequences[]
  diagnostic_counts[]
  coverage

RecordShape
  discriminator_path?
  discriminator_value?
  field_types[]
  counter_paths[]
  timestamp_path?
  session_key_path?
  event_key_path?
  sampled_record_count

CounterSequence
  field_path
  observed_behavior: per_event | monotonic | reset_observed | uncertain
  synthetic_values[]

Coverage
  files_considered
  complete_records_considered
  byte_limit_reached
  record_limit_reached
  supported_shape_found
```

`layout_patterns` are provider-relative patterns with every dynamic segment replaced by a fixed marker. `field_types` contain JSON paths and types only. Identifier values are never emitted; reports record only the relevant field path, type, presence, and uniqueness count. Actual timestamps are replaced by fixed synthetic timestamps while preserving order.

## Discovery and sampling

Discovery enumerates only the known native Windows root for the requested provider. It reads file name, type, size, and modification metadata to choose recent candidates without scanning unrelated user directories.

Inspection streams selected JSON or JSONL files and stops per provider when it has sufficient evidence for token-bearing record shapes or reaches any hard limit:

- Five source files.
- 50 MiB of source bytes.
- 50,000 complete records.

An incomplete final JSONL line is classified as pending and excluded. Malformed complete records contribute a bounded diagnostic category and count but no record data.

The probe stops adding files after it has observed the token-bearing shapes available in the newest relevant session material and enough ordered numeric observations to classify each discovered counter path. If the limits arrive first, the outcome is `limit_reached` and the report states which evidence remains incomplete.

## Sanitization

Raw records exist only as bounded in-process values during streaming inspection. They are never printed, logged, returned through Tauri, or serialized directly.

Sanitization preserves only information required to build provider adapters:

- JSON field paths and value types.
- Bounded record discriminator values approved as structural metadata.
- Paths to token counters, timestamps, session keys, and event keys.
- Synthetic numeric sequences preserving observed counter behavior.
- Synthetic timestamps preserving ordering.
- Fixed synthetic identifiers preserving equality relationships.
- Provider-relative layout patterns with dynamic segments removed.

Candidate fixtures preserve the nesting required to reach structural and token fields. All other source values are omitted. A fixed synthetic unknown field is added separately so future adapter tests can prove unknown fields are ignored without copying an unknown source value.

## Privacy validator

Sanitized artifacts are validated before the first write. Validation is fail-closed and checks that:

- No absolute Windows path, UNC path, URI, repository path, or working directory is present.
- No real identifier or timestamp is present.
- No non-allow-listed source string survives sanitization.
- No prompt, response, reasoning, tool payload, credential, or source-code field is emitted.
- Every structural string is bounded and every numeric token value is a non-negative integer.
- Fixture files contain only the approved structural paths plus fixed synthetic values.

Inspection maintains a bounded hash ledger of source strings. The validator hashes every output string and rejects a match unless the value is an explicitly allow-listed structural discriminator. On failure, the candidate output directory is removed and the process exits non-zero. Error output contains the provider, validation category, and count only.

## Committed compatibility artifacts

For each provider, `manifest.json` records the sanitized layout pattern, discovered structural paths, interpreted counter behavior, event/session identity paths, timestamp path, and fixture record count. `records.jsonl` contains synthetic records representing the observed token-bearing shapes and ordered counter behavior.

The compatibility report summarizes:

- Whether the native provider was detected and readable.
- Provider-relative file-layout patterns.
- Token-bearing discriminator and field paths.
- Observed counter behavior and any uncertainty.
- Stable-key and timestamp field paths.
- Probe limits and coverage.
- Fixture paths and privacy-validation result.

The report contains no absolute source root, source file name, real identifier, real timestamp, repository path, or conversational field value.

## Provider outcomes

Each requested provider completes independently with one outcome:

- `detected`: supported token-bearing shapes were found and sanitized.
- `not_detected`: the known native root does not exist.
- `permission_denied`: the native root exists but cannot be read.
- `unsupported_format`: readable records exist but no supported token structure is identifiable.
- `limit_reached`: hard limits were exhausted before sufficient evidence was collected.

One provider's outcome does not suppress the other provider's report. Privacy-validation failure is the only fail-fast condition because no output may be trusted after a boundary violation.

## Testing

Tests run the Cargo example against synthetic temporary profile roots and assert:

- Native-root resolution remains beneath the supplied profile root.
- Candidate selection obeys file, byte, and record limits.
- JSON and JSONL shapes are inventoried without copying raw values.
- Incremental and cumulative sequences preserve behavior after sanitization.
- Counter resets remain distinguishable.
- Real identifiers, absolute paths, timestamps, prompt-like text, secrets, and oversized strings cannot appear in output.
- Malformed complete lines increment bounded categories.
- Partial final lines remain pending.
- Missing, blocked, unsupported, and limited providers remain independent.
- Generated reports, manifests, and JSONL fixtures parse successfully.

The full gate also runs the existing Rust tests and compilation checks. The live smoke run is:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --example provider_probe -- all
```

## Execution topology

Execution uses staged Luna Max delegation:

1. One Luna Max agent implements the shared probe and synthetic tests.
2. The root agent reviews the privacy boundary and runs the shared gate.
3. Two Luna Max agents run the validated probe and produce sanitized Claude Code and Codex compatibility artifacts in parallel.
4. One Luna Max integration pass validates both fixture sets and the compatibility report.
5. The root agent independently reviews repository changes, reruns all gates, and reports any provider that could not be characterized.

No agent receives raw session records in its prompt or tool output. Agents consume only the probe's validated sanitized artifacts.

## Acceptance criteria

This slice is complete when:

1. The probe is development-only and absent from the shipped Tauri command surface.
2. Discovery is limited to known native Windows provider roots.
3. Synthetic tests prove the sampling and privacy boundaries.
4. A live run produces an independent sanitized outcome for Claude Code and Codex.
5. Every detected provider has a parseable manifest and synthetic JSONL fixture set.
6. The compatibility report records layouts, structural paths, counter behavior, identity paths, timestamps, coverage, and uncertainties without private content.
7. Privacy validation proves that raw paths, identifiers, timestamps, and conversational values are absent from committed artifacts.
8. Existing Rust and frontend bootstrap checks remain green.
