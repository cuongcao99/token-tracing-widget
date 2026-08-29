# Collection Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect bounded provider-file discovery to the Rust adapters, normalized usage processing, restart-safe SQLite persistence, and post-commit usage summaries.

**Architecture:** A Rust-only `CollectionCoordinator` receives private discovered-file handles and persisted checkpoints, invokes Claude and Codex adapters independently, and converts normalized observations into deduplicated usage events. A single SQLite transaction writes accepted events, session state, source health, diagnostics, and file checkpoints; the summary is queried only after commit. The watcher, frontend, Tauri event wiring, and new commands remain outside this slice.

**Tech Stack:** Rust 2021, Tauri 2 host, `serde`/`serde_json`, direct SQLite through `rusqlite` (no ORM), existing React/TypeScript/Vite build for final regression checks.

**Spec:** `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`

## Global Constraints

- Version one remains local-only and Windows 11-only.
- Tauri 2 is the desktop shell and Rust owns filesystem and SQLite access.
- SQLite is accessed only by the Rust core; React receives no database handle or raw observation.
- Adapters emit normalized metadata only: provider, opaque session/event keys, timestamp, counter kind, token counters.
- Prompts, responses, reasoning, tool payloads, credentials, repository contents, raw JSON, and working directories never enter normalized events, SQLite, diagnostics, or frontend payloads.
- Incremental observations are stored once; cumulative observations are ordered and converted to deltas; cumulative values are never summed directly.
- A cumulative decrease starts a new monotonic segment and never creates a negative delta.
- Stable event identities survive restart, rescan, truncation, and file rotation.
- Claude and Codex source health stays independent.
- No network client, telemetry, sidecar, background service, frontend state library, CSS framework, ORM, new Tauri command, WSL auto-discovery, or watcher integration is added in this slice.
- The existing untracked plan `docs/superpowers/plans/2026-08-29-source-discovery.md` is preserved.

## File Map

- Modify `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` to add the direct SQLite driver required by the existing Rust storage boundary.
- Modify `src-tauri/src/providers/provider_adapter.rs` and `src-tauri/src/utils/bounded_io.rs` to carry source positions and preserve incomplete final JSONL lines.
- Modify `src-tauri/src/sources/session_files.rs` only to expose Rust-internal file metadata needed to create an opaque identity; keep the filesystem path private and non-serializable.
- Complete `src-tauri/src/types/file_checkpoint.rs`, `src-tauri/src/types/usage_event.rs`, `src-tauri/src/types/source_health.rs`, and `src-tauri/src/types/usage_summary.rs`.
- Complete `src-tauri/src/usage/observation_validation.rs`, `src-tauri/src/usage/cumulative_delta.rs`, `src-tauri/src/usage/duplicate_event_filter.rs`, `src-tauri/src/usage/active_provider.rs`, and `src-tauri/src/usage/daily_total.rs`.
- Complete `src-tauri/src/database/schema.rs`, `connection.rs`, `usage_events.rs`, `file_checkpoints.rs`, `sessions.rs`, `sources.rs`, `diagnostics.rs`, and `checkpoints.rs`.
- Create `src-tauri/src/collection/mod.rs` and register it from `src-tauri/src/lib.rs`.
- Add `src-tauri/tests/collection_core.rs` and `src-tauri/tests/database.rs`; extend `src-tauri/tests/provider_readers.rs` for the changed reader contract.
- Do not modify `src-tauri/src/sources/file_watcher.rs`, React state, frontend components, or Tauri command surface.

### Task 1: Freeze the private file/checkpoint/reader contract

**Files:**
- Modify: `src-tauri/src/providers/provider_adapter.rs`
- Modify: `src-tauri/src/utils/bounded_io.rs`
- Modify: `src-tauri/src/sources/session_files.rs`
- Modify: `src-tauri/src/types/file_checkpoint.rs`
- Test: `src-tauri/tests/provider_readers.rs`
- Test: `src-tauri/tests/collection_core.rs`

**Interfaces:**
- `ProviderReadObservation` contains one `TokenObservation` and its zero-based record start offset; the offset is internal metadata, not a frontend field.
- `ProviderReadResult` contains `observations: Vec<ProviderReadObservation>`, `next_offset: u64`, and `pending_offset: Option<u64>`.
- `FileCheckpoint` contains `provider`, an opaque SHA-256 file identity, `byte_offset`, `size_bytes`, `modified_at_unix_ms`, `monotonic_segment`, and the last cumulative counters needed for restart-safe delta conversion.
- `DiscoveredSessionFile` retains its private absolute path and exposes only Rust-internal size, modification time, kind, and path accessors.

- [ ] **Step 1: Write failing tests for source positions and partial final lines.**

```rust
#[test]
fn incomplete_final_line_stays_pending_until_completed() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{{\"message\":{{\"type\":\"message\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":10}}}},\"timestamp\":\"2026-01-01T00:00:00Z\"}}\n{{\"message\":").unwrap();

    let first = ClaudeReader::default()
        .read_observations(file.path(), 0)
        .expect("complete records must be readable");

    assert_eq!(first.observations.len(), 1);
    assert_eq!(first.next_offset, first.pending_offset.unwrap());

    write!(file, "{{\"type\":\"message\",\"usage\":{{\"input_tokens\":20,\"output_tokens\":20}}}},\"timestamp\":\"2026-01-01T00:00:01Z\"}}\n").unwrap();
    let second = ClaudeReader::default()
        .read_observations(file.path(), first.next_offset)
        .expect("completed record must be readable");

    assert_eq!(second.observations.len(), 1);
    assert!(second.pending_offset.is_none());
}
```

- [ ] **Step 2: Run the focused tests and verify the contract fails.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test provider_readers --test collection_core incomplete_final_line_stays_pending_until_completed`

Expected: FAIL because the current result has no `pending_offset`, no source position, and treats an incomplete final JSONL line as `InvalidJson`.

- [ ] **Step 3: Implement the minimal reader contract.**

Use a bounded line result with a `terminated` flag. Parse an unterminated final line when it is valid JSON; when parsing fails, leave `next_offset` at that line's start and return `pending_offset: Some(start)`. A newline-terminated malformed record still returns the existing sanitized `InvalidJson` error. Record start offsets are captured before consuming each line.

Create the opaque identity from provider plus the private filesystem path using SHA-256. Never serialize, log, or include the source path in `FileCheckpoint`.

- [ ] **Step 4: Run reader and collection-core tests.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test provider_readers --test collection_core`

Expected: PASS, with existing provider parsing assertions unchanged except for the new internal result fields.

- [ ] **Step 5: Commit the isolated contract change.**

```text
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/providers/provider_adapter.rs src-tauri/src/utils/bounded_io.rs src-tauri/src/sources/session_files.rs src-tauri/src/types/file_checkpoint.rs src-tauri/tests/provider_readers.rs src-tauri/tests/collection_core.rs
git commit -m "feat: define collection file checkpoint contract"
```

### Task 2: Implement validation, event identity, and cumulative deltas

**Files:**
- Modify: `src-tauri/src/types/token_observation.rs`
- Modify: `src-tauri/src/types/usage_event.rs`
- Modify: `src-tauri/src/usage/observation_validation.rs`
- Modify: `src-tauri/src/usage/cumulative_delta.rs`
- Modify: `src-tauri/src/usage/duplicate_event_filter.rs`
- Test: `src-tauri/tests/collection_core.rs`

**Interfaces:**
- `validate_observation(&TokenObservation) -> Result<(), ObservationValidationError>` rejects negative/impossible counters, missing timestamps, and `total_tokens` values that disagree with available input plus output.
- `UsageEvent` stores only `event_id`, provider, opaque file identity, effective opaque session key, source position, observed timestamp, counter kind, monotonic segment, and token deltas.
- `convert_observations(file_identity, checkpoint, observations) -> Result<DeltaBatch, CollectionError>` sorts by timestamp then source position, converts incremental observations directly, converts cumulative observations against checkpoint state, and returns the updated checkpoint.
- `event_id(provider, file_identity, observation, source_position) -> String` uses the provider event key when present and otherwise uses file identity plus source position plus counter kind; it never uses raw record content.
- A missing Codex session key falls back to the opaque file identity so current-session aggregation remains possible without exposing a path.

- [ ] **Step 1: Write failing tests for validation and delta rules.**

```rust
#[test]
fn cumulative_snapshots_become_deltas_and_reset_starts_new_segment() {
    let observations = vec![
        ProviderReadObservation::new(codex_observation("2026-01-01T00:00:00Z", 10), 0),
        ProviderReadObservation::new(codex_observation("2026-01-01T00:00:01Z", 20), 100),
        ProviderReadObservation::new(codex_observation("2026-01-01T00:00:02Z", 5), 200),
    ];
    let checkpoint = FileCheckpoint::new("file-a", Provider::Codex);
    let batch = convert_observations("file-a", &checkpoint, observations).unwrap();

    assert_eq!(batch.events.iter().map(|event| event.total_tokens).collect::<Vec<_>>(), vec![10, 10, 5]);
    assert_eq!(batch.events[0].monotonic_segment, 0);
    assert_eq!(batch.events[2].monotonic_segment, 1);
}

#[test]
fn duplicate_stable_event_key_is_accepted_once() {
    let observation = TokenObservation {
        provider: Provider::Claude,
        source_session_key: Some("session-a".to_owned()),
        source_event_key: Some("event-1".to_owned()),
        observed_at: "2026-01-01T00:00:00Z".to_owned(),
        counter_kind: CounterKind::Incremental,
        input_tokens: Some(10),
        cached_input_tokens: Some(4),
        output_tokens: Some(10),
        total_tokens: 20,
    };
    let observations = vec![ProviderReadObservation::new(observation, 0)];
    let checkpoint = FileCheckpoint::new("file-a", Provider::Claude);
    let first = convert_observations("file-a", &checkpoint, observations.clone()).unwrap();
    let second = convert_observations("file-a", &first.next_checkpoint, observations).unwrap();

    assert_eq!(first.events.len(), 1);
    assert!(second.events.is_empty());
}
```

The test module defines `codex_observation(timestamp, total)` as a test-only constructor that sets input, output, and total to `total`, cached input to `Some(total)`, and the other normalized fields to the Codex cumulative fixture values. `ProviderReadObservation::new` stores the supplied observation and source position.

- [ ] **Step 2: Run focused tests and verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test collection_core cumulative_snapshots_become_deltas_and_reset_starts_new_segment`

Expected: FAIL because the usage, checkpoint, and deduplication modules are still stubs.

- [ ] **Step 3: Implement the smallest pure functions.**

Validate all available numeric fields as unsigned non-negative values. If input and output are both present, require `total_tokens == input_tokens + output_tokens` with checked arithmetic. Never add cached input to total. For cumulative observations, compare total/input/output counters with the previous segment; emit the current counter values as the first delta in a new segment after any decrease. Do not emit negative values.

- [ ] **Step 4: Add ordering and identity regression cases.**

Cover out-of-order timestamps, same timestamp with source-position ordering, missing source keys, duplicate scans, duplicate provider event keys, cached input changes, overflow rejection, and unknown record kinds being ignored.

- [ ] **Step 5: Run the pure collection-core tests.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test collection_core`

Expected: PASS with no filesystem or SQLite access from the pure usage functions.

- [ ] **Step 6: Commit the pure collection rules.**

```text
git add src-tauri/src/types/token_observation.rs src-tauri/src/types/usage_event.rs src-tauri/src/usage/observation_validation.rs src-tauri/src/usage/cumulative_delta.rs src-tauri/src/usage/duplicate_event_filter.rs src-tauri/tests/collection_core.rs
git commit -m "feat: normalize and deduplicate usage deltas"
```

### Task 3: Add parameterized SQLite persistence with atomic event/checkpoint writes

**Files:**
- Modify: `src-tauri/src/database/schema.rs`
- Modify: `src-tauri/src/database/connection.rs`
- Modify: `src-tauri/src/database/usage_events.rs`
- Modify: `src-tauri/src/database/file_checkpoints.rs`
- Modify: `src-tauri/src/database/checkpoints.rs`
- Modify: `src-tauri/src/database/sessions.rs`
- Modify: `src-tauri/src/database/sources.rs`
- Modify: `src-tauri/src/database/diagnostics.rs`
- Create: `src-tauri/tests/database.rs`

**Interfaces:**
- `IndexStore::open(path: &Path) -> Result<IndexStore, StorageError>` opens the local SQLite file and applies idempotent schema creation.
- `IndexStore::load_checkpoint(identity: &str) -> Result<Option<FileCheckpoint>, StorageError>` returns only normalized checkpoint metadata.
- `IndexStore::apply_batch(&mut self, batch: &CollectionBatch) -> Result<(), StorageError>` inserts new usage events, upserts sessions/source health/diagnostics, and upserts file checkpoints inside one transaction.
- `IndexStore::query_events_for_summary(day_start: &str, now: &str) -> Result<SummaryRows, StorageError>` reads normalized columns only.
- Test-only `IndexStore::count_usage_events() -> Result<u64, StorageError>` counts rows without exposing row contents.
- `CollectionBatch::new(events: Vec<UsageEvent>, checkpoints: Vec<FileCheckpoint>) -> CollectionBatch` creates the transaction input.
- `FileCheckpoint::with_position(identity: &str, provider: Provider, byte_offset: u64, size_bytes: u64) -> FileCheckpoint` creates a checkpoint fixture with zero segment and no prior cumulative state.
- All SQL uses bound parameters; no raw JSON or provider-file body is accepted by any database function.

- [ ] **Step 1: Write failing persistence tests.**

```rust
#[test]
fn event_and_checkpoint_commit_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = IndexStore::open(&directory.path().join("index.sqlite")).unwrap();
    let event = test_usage_event("event-1", "file-a");
    let checkpoint = FileCheckpoint::with_position("file-a", Provider::Claude, 42, 42);
    let batch = CollectionBatch::new(vec![event], vec![checkpoint]);

    database.apply_batch(&batch).unwrap();

    assert_eq!(database.count_usage_events().unwrap(), 1);
    assert_eq!(database.load_checkpoint("file-a").unwrap().unwrap().byte_offset, 42);
}

#[test]
fn failed_batch_rolls_back_event_and_checkpoint_together() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = IndexStore::open(&directory.path().join("index.sqlite")).unwrap();
    let event = test_usage_event("event-1", "file-new");
    let invalid_checkpoint = FileCheckpoint::with_position("file-new", Provider::Claude, 43, 42);
    let batch = CollectionBatch::new(vec![event], vec![invalid_checkpoint]);

    assert!(database.apply_batch(&batch).is_err());
    assert_eq!(database.count_usage_events().unwrap(), 0);
    assert!(database.load_checkpoint("file-new").unwrap().is_none());
}
```

The test module defines `test_usage_event(event_id, file_identity)` with provider Claude, session key `session-a`, source position `0`, timestamp `2026-01-01T00:00:00Z`, incremental counters input `10`, cached input `4`, output `10`, and total `20`. The schema adds `CHECK(byte_offset <= size_bytes)` so the second test fails after the event write and proves the transaction rolls back both writes.

- [ ] **Step 2: Run database tests and verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test database`

Expected: FAIL because the database modules are empty and `Cargo.toml` has no SQLite driver.

- [ ] **Step 3: Add the direct SQLite driver and schema.**

Use `rusqlite` directly. Create `sources`, `sessions`, `usage_events`, `file_checkpoints`, and `diagnostics` with primary keys for provider/file/event identity, integer token columns, and no raw-record column. Store only the explicitly configured source root in `sources`; never expose it through the overlay summary.

- [ ] **Step 4: Implement the transaction boundary.**

Start one transaction in `apply_batch`, write every accepted event and its session/source/diagnostic changes, then write the checkpoint last. Return only after `commit()` succeeds. On any constraint or I/O error, roll back and return a sanitized `StorageError` category.

- [ ] **Step 5: Run persistence and privacy tests.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test database`

Expected: PASS, including a schema inspection that finds no prompt, response, reasoning, tool, credential, repository, working-directory, or raw-record column.

- [ ] **Step 6: Commit the SQLite slice.**

```text
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/database src-tauri/tests/database.rs
git commit -m "feat: persist usage events atomically"
```

### Task 4: Implement independent provider collection coordination

**Files:**
- Create: `src-tauri/src/collection/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/sources/session_files.rs`
- Modify: `src-tauri/src/types/source_health.rs`
- Test: `src-tauri/tests/collection_core.rs`

**Interfaces:**
- `ProviderSource::new(enabled: bool, discovery: DiscoveryResult, adapter: &'a dyn ProviderAdapter) -> ProviderSource<'a>` contains an enabled flag, one discovery result, and one adapter; the filesystem path stays private inside Rust.
- `CollectionStore` abstracts checkpoint loading, batch application, and post-commit summary rows; `IndexStore` is its production implementation and an in-memory test store is used for coordinator unit tests.
- `CollectionCoordinator<S>::new(store: S) -> CollectionCoordinator<S>` where `S: CollectionStore` creates a coordinator with an initial `Loading` summary; `collect(&mut self, sources: &[ProviderSource<'_>], clock: &dyn CollectionClock) -> Result<CollectionReport, CollectionError>` processes both providers in deterministic order.
- `CollectionCoordinator::last_summary(&self) -> &UsageSummary` returns the last committed summary or `Stale` after a storage failure.
- `CollectionClock` exposes `now() -> &str` and `local_day() -> &str`; `FixedClock::new(now: &str, local_day: &str) -> FixedClock` is the test implementation.
- `StorageError::Write` is the sanitized write-failure category used by `FailingStore`.
- `CollectionReport` contains `UsageSummary`, accepted-event count, and per-provider `SourceHealth`; it contains no observations, paths, raw records, or adapter payloads.
- Provider read/parse failures become that provider's sanitized health/diagnostic result and do not prevent the other provider from being collected.

- [ ] **Step 1: Write failing coordinator tests.**

```rust
#[test]
fn one_provider_failure_does_not_block_the_other_provider() {
    let (mut coordinator, sources) = test_sources_with_one_broken_codex();
    let report = coordinator
        .collect(&sources, &FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"))
        .unwrap();

    assert_eq!(report.summary.today_tokens, 20);
    assert_eq!(report.summary.source_health[0].state, "detected");
    assert_eq!(report.summary.source_health[1].state, "unavailable");
}

#[test]
fn summary_is_not_recomputed_when_sqlite_commit_fails() {
    let (mut coordinator, sources) = test_sources_with_failing_store();
    let result = coordinator.collect(
        &sources,
        &FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"),
    );

    assert!(matches!(result, Err(CollectionError::Storage(StorageError::Write))));
    assert_eq!(coordinator.last_summary().state, UsageState::Stale);
}
```

The test module defines `test_sources_with_one_broken_codex()` using the existing synthetic Claude fixture and an `AlwaysFailReader` whose `read_observations` returns `ProviderReadError::Io`; it returns `(CollectionCoordinator<InMemoryStore>, Vec<ProviderSource>)`. `test_sources_with_failing_store()` uses the same sources with `FailingStore::new(StorageError::Write)`. `FixedClock::new(now, local_day)` implements `CollectionClock` with those exact strings. The in-memory and failing stores implement every method in `CollectionStore` and keep the last summary so the test can assert that a failed commit leaves `Stale` state.

- [ ] **Step 2: Run coordinator tests and verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test collection_core one_provider_failure_does_not_block_the_other_provider`

Expected: FAIL because no coordinator or source-health implementation exists.

- [ ] **Step 3: Implement provider-independent collection.**

For each enabled provider, load each file checkpoint, reset the byte offset when the file shrank or its identity changed, call the matching adapter, convert observations into deltas, and record a pending tail without advancing past it. Map discovery/read outcomes to `detected`, `not_detected`, `permission_denied`, `invalid_root`, `unavailable`, `limited`, `malformed`, or `unsupported_format`.

- [ ] **Step 4: Apply one batch transaction and query only after commit.**

Merge accepted events and updated checkpoints, call `IndexStore::apply_batch`, then query the summary. If persistence fails, keep the previous totals and return `stale`; do not emit or return a fresh summary from uncommitted data.

- [ ] **Step 5: Add restart, append, rotation, and partial-write integration cases.**

Cover first scan, second scan with the same checkpoint, append after checkpoint, file truncation, file rotation with duplicate event keys, incomplete final line followed by completion, concurrent Claude/Codex inputs, and a failing provider alongside a valid provider.

- [ ] **Step 6: Run the integrated Rust suite.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --all-targets`

Expected: all existing provider/source-discovery tests plus collection and database tests pass.

- [ ] **Step 7: Commit the coordinator slice.**

```text
git add src-tauri/src/collection src-tauri/src/lib.rs src-tauri/src/providers/mod.rs src-tauri/src/sources/session_files.rs src-tauri/src/types/source_health.rs src-tauri/tests/collection_core.rs
git commit -m "feat: coordinate local provider collection"
```

### Task 5: Add post-commit active-session and local-day summary rules

**Files:**
- Modify: `src-tauri/src/usage/active_provider.rs`
- Modify: `src-tauri/src/usage/daily_total.rs`
- Modify: `src-tauri/src/types/usage_summary.rs`
- Modify: `src-tauri/src/types/source_health.rs`
- Modify: `src-tauri/src/utils/windows_time.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/collection_core.rs`

**Interfaces:**
- `CollectionClock` supplies a testable current instant and current Windows local calendar day; production uses the Windows local clock, tests use a fixed clock.
- `UsageEvent::for_test(provider: Provider, session_key: &str, observed_at: &str, total_tokens: u64) -> UsageEvent` creates a normalized test event with matching input/output counters and a unique event identity.
- `SourceHealth::detected(provider: Provider) -> SourceHealth` creates a detected test health row with the existing serialized provider/state field names.
- `compute_active_provider(events: &[UsageEvent], now: &str) -> ActiveProviderResult` selects the provider with the newest valid event in the preceding two minutes; no qualifying event yields `Idle` while retaining the last-update timestamp.
- `compute_today_total(events: &[UsageEvent], local_day: &str) -> u64` sums accepted deltas whose timestamps fall in the current Windows local calendar day across enabled providers.
- `compute_summary(events: &[UsageEvent], source_health: &[SourceHealth], clock: &dyn CollectionClock) -> UsageSummary` combines those two pure results with source health and current-session selection.
- `UsageSummary` serializes only `state`, optional `provider`, optional `currentSessionTokens`, `todayTokens`, optional `lastUpdatedAt`, and `sourceHealth`.

- [ ] **Step 1: Write failing summary tests.**

```rust
#[test]
fn active_provider_expires_after_two_minutes_but_last_update_remains() {
    let events = vec![UsageEvent::for_test(
        Provider::Claude,
        "session-a",
        "2026-01-01T10:00:00Z",
        20,
    )];
    let source_health = vec![SourceHealth::detected(Provider::Claude)];
    let summary = compute_summary(
        &events,
        &source_health,
        &FixedClock::new("2026-01-01T10:02:01Z", "2026-01-01"),
    );

    assert_eq!(summary.state, UsageState::Idle);
    assert_eq!(summary.last_updated_at.as_deref(), Some("2026-01-01T10:00:00Z"));
    assert_eq!(summary.today_tokens, 20);
}

#[test]
fn today_total_combines_enabled_providers_without_double_counting_cumulative_snapshots() {
    let events = vec![
        UsageEvent::for_test(Provider::Claude, "session-a", "2026-01-01T10:00:00Z", 20),
        UsageEvent::for_test(Provider::Codex, "file-b", "2026-01-01T10:00:01Z", 20),
    ];
    let source_health = vec![
        SourceHealth::detected(Provider::Claude),
        SourceHealth::detected(Provider::Codex),
    ];
    let summary = compute_summary(
        &events,
        &source_health,
        &FixedClock::new("2026-01-01T10:00:30Z", "2026-01-01"),
    );

    assert_eq!(summary.today_tokens, 40);
}
```

- [ ] **Step 2: Run summary tests and verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test collection_core active_provider_expires_after_two_minutes_but_last_update_remains`

Expected: FAIL because the active-provider, local-day, and typed summary modules are still stubs.

- [ ] **Step 3: Implement summary queries over committed normalized events.**

Use parsed timestamps and the injected local-day boundary. Select current-session totals from the latest active session; include all enabled providers in today's total. Preserve the last valid update when state becomes idle or stale. Do not add raw event fields to the summary.

- [ ] **Step 4: Preserve the existing bootstrap command behavior.**

Keep the current `get_usage_summary` Tauri command and its public serialized field names. This slice may provide the typed summary to internal callers, but it does not add a command, event listener, frontend state, or polling path.

- [ ] **Step 5: Run all Rust tests and privacy assertions.**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `cargo test --manifest-path src-tauri/Cargo.toml --all-targets`

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: formatting, all Rust tests, and type checking pass; serialized summaries contain no forbidden raw fields.

- [ ] **Step 6: Commit the summary slice.**

```text
git add src-tauri/src/usage/active_provider.rs src-tauri/src/usage/daily_total.rs src-tauri/src/types/usage_summary.rs src-tauri/src/types/source_health.rs src-tauri/src/utils/windows_time.rs src-tauri/src/lib.rs src-tauri/tests/collection_core.rs
git commit -m "feat: compute committed usage summaries"
```

### Task 6: Run repository completion gates without expanding scope

**Files:**
- No source changes unless a failing gate identifies a regression in Tasks 1-5.

- [ ] **Step 1: Run frontend regression checks.**

Run: `npm test -- --run`

Run: `npm run build`

Expected: existing frontend tests and Vite production build pass without adding frontend state.

- [ ] **Step 2: Run the integrated Tauri build.**

Run: `npm run tauri build -- --debug --no-bundle`

Expected: `src-tauri/target/debug/token-tracing-widget.exe` is produced; no app-managed sidecar or background service appears.

- [ ] **Step 3: Run the privacy-boundary review.**

Inspect the SQLite schema, diagnostic strings, `CollectionReport`, and serialized `UsageSummary`. Confirm only normalized counters, opaque identities, timestamps, statuses, and bounded categories are present. Confirm absolute discovered paths remain Rust-private.

- [ ] **Step 4: Leave watcher, WSL auto-discovery, frontend wiring, and new commands for a later approved slice.**

Stop after the collection contracts, transaction boundary, and post-commit summary tests pass. Do not use `src-tauri/src/sources/file_watcher.rs` in this plan.

## Self-Review Against the Spec

- Source discovery boundary: preserved; the coordinator consumes discovered files and never scans arbitrary directories.
- Provider adapters: preserved behind one shared interface; only normalized metadata crosses the adapter boundary.
- Collection core: validation, ordering, cumulative delta conversion, monotonic segments, deduplication, checkpointing, and active-session selection are covered by Tasks 1, 2, 4, and 5.
- Storage: normalized tables, parameterized queries, atomic event/checkpoint writes, independent health, and bounded diagnostics are covered by Task 3.
- Data flow: adapter output is committed before summary recomputation; partial final lines remain pending.
- Privacy: no raw record body, conversational content, credentials, repository contents, or working directory enters storage or UI payloads.
- Testing: Rust unit/integration/privacy checks, frontend regression checks, and an integrated Windows Tauri build are covered by Tasks 1-6.
- Explicitly deferred: watcher integration, WSL auto-discovery, new Tauri commands/events, frontend state, network, telemetry, sidecars, and UI behavior.
