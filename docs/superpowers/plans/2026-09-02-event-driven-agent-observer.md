# Event-driven agent observer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make agent lifecycle hooks start and stop provider observation without blocking hook delivery behind filesystem collection, while preserving restart-safe metadata-only token accounting.

**Architecture:** Keep one permanent hook ingress and one serialized SQLite collection worker, but move lifecycle handling into a separate control loop. The control loop owns an in-memory multi-session registry and dynamically leases one native `SourceObserver` per active provider. Hooks trigger immediate lifecycle summaries and collection commands; provider files remain the only token-accounting source.

**Tech Stack:** Rust 2021, Tauri 2, Win32 FFI already present, `std::sync::mpsc`, `rusqlite`, serde, React 19, TypeScript, Vitest, Vite, and plain CSS. No new dependency.

**Spec:** `docs/superpowers/specs/2026-09-02-event-driven-agent-observer-design.md`

## Global Constraints

- Version 1 remains local-only and Windows 11-only.
- Rust owns all filesystem and SQLite access; React receives typed `UsageSummary` values only.
- Hooks are lifecycle hints and never create `UsageEvent` rows or token totals.
- Provider session files remain the token-accounting source and restart-recovery source.
- Prompts, responses, reasoning, tool payloads, credentials, repository contents, working directories, raw JSON, and absolute source paths never enter normalized events, SQLite, diagnostics, or frontend/event payloads.
- Existing provider adapters, cumulative-to-delta conversion, deduplication, checkpoints, one event/checkpoint transaction, source health, and summary wire contract remain in use.
- The existing 120-second event window remains the fallback for providers that have not received a hook in the current process; an observed provider uses lifecycle state while its runs are controlled.
- Stop publishes lifecycle idle immediately when its provider has no remaining active run, then requests one bounded final flush.
- There is no observer or reconciliation schedule when no provider run is active.
- One collection worker remains the only SQLite writer.
- Do not add a watcher crate, frontend state library, CSS framework, ORM, network client, telemetry, sidecar, background service, or new Tauri capability.

---

## File map

- Create: `docs/superpowers/specs/2026-09-02-event-driven-agent-observer-design.md` — approved architecture and invariants.
- Create: `docs/superpowers/plans/2026-09-02-event-driven-agent-observer.md` — this executable plan.
- Modify: `src-tauri/src/app/runtime.rs` — separate long-running collection state from in-memory lifecycle state, support multiple runs, and compose summaries without holding the collection lock.
- Modify: `src-tauri/src/app/live_collection.rs` — split control and collection responsibilities, add lifecycle-aware scheduling, final flush, and one collection writer.
- Modify: `src-tauri/src/app/trace_signal.rs` — keep hook ingress on the control channel and add bounded named-pipe delivery retry.
- Modify: `src-tauri/src/sources/file_watcher.rs` — expose `SourceObserver` with dynamic per-provider start/stop and provider leases while retaining the native Win32 implementation.
- Unchanged: `src-tauri/src/sources/mod.rs` and `src-tauri/src/lib.rs` — the
  existing public module and startup/exit ownership already route through the
  live runtime entry point.
- Modify: `src-tauri/src/types/trace_signal.rs` only if a validation invariant is needed; do not add raw fields or a database contract.
- Test: Rust unit/integration tests at the registry, live-loop, observer, trace-signal, collection, and privacy seams.
- Do not modify: `src-tauri/src/database/schema.rs`, database table shape, React files, or frontend contracts.

## Task 1: Add the multi-run lifecycle registry and decoupled summary seam

**Files:**
- Modify: `src-tauri/src/app/runtime.rs`
- Test: `src-tauri/src/app/runtime.rs` (`#[cfg(test)]`)

**Interfaces:**
- `AppState::apply_trace_signal(&self, signal: &TraceSignal, received_at: Instant) -> Result<TraceSignalResult, RuntimeError>` updates only the fast lifecycle state and returns an immediate summary plus a transition.
- `TraceSignalResult` contains `summary: UsageSummary` and a transition with
  `Started { provider, first_run }`, `Stopped { provider, last_run }`, or
  `Ignored`.
- `AppState::summary() -> UsageSummary` composes the last post-collection base summary with the current lifecycle snapshot.
- `AppState::collect_once(&self, clock: &dyn CollectionClock) -> Result<CollectionReport, RuntimeError>` updates the base summary after collection while lifecycle state remains independently lockable.

- [x] **Step 1: Write the failing registry tests.**

Add tests for these observable behaviors through `AppState`:

```rust
#[test]
fn two_sessions_share_active_provider_until_both_stop() {}

#[test]
fn stale_stop_does_not_end_newer_turn_generation() {}

#[test]
fn stop_for_last_run_forces_idle_even_when_token_event_is_recent() {}

#[test]
fn lifecycle_signal_does_not_wait_for_collection_runtime_lock() {}
```

Use the existing temporary profile/database helpers and synthetic metadata-only records. The tests must assert totals remain unchanged, the immediate Active summary does not change `last_updated_at`, and Stop does not add database rows.

- [x] **Step 2: Run the focused tests and confirm the new seam fails.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib app::runtime --offline
```

Expected: the new transition/multi-run assertions fail because the runtime currently stores one optional trace activity and holds the same runtime mutex during collection.

- [x] **Step 3: Implement the smallest decoupled runtime state.**

Move the base collection summary and lifecycle registry behind separate shared locks. Replace the single `Option<TraceActivity>` with a map keyed by provider plus optional bounded session identity. Store `hooked_providers` separately so a provider that has received a hook can be forced Idle after its last Stop. Use the current turn identity as a generation fence and use `Instant` only in memory.

Keep `Runtime::collect_once` responsible for provider discovery, adapters, transactions, and event-derived base summaries. Do not add a hook table, hook fields to `UsageEvent`, or a second aggregation path.

- [x] **Step 4: Run the focused tests and the existing runtime tests.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib app::runtime --offline
cargo test --manifest-path src-tauri/Cargo.toml --test runtime_integration --offline
```

Expected: the new registry tests and all existing runtime tests pass with no schema changes.

- [x] **Step 5: Commit the runtime seam.**

```powershell
git add src-tauri/src/app/runtime.rs
git commit -m "feat: track multiple hook runs independently"
```

## Task 2: Split control and collection lanes

**Files:**
- Modify: `src-tauri/src/app/live_collection.rs`
- Test: `src-tauri/src/app/live_collection.rs` (`#[cfg(test)]`)

**Interfaces:**
- `CollectionCommand` is internal and contains `Activate(Provider)`,
  `Changed(Provider)`, `WatchUnavailable(Provider)`, `ConfigurationChanged`,
  `Finalize { provider, last_run }`, and `Shutdown`.
- `LiveCollectionLoop::run(receiver: Receiver<CollectionCommand>)` is the only loop allowed to call `CollectionBackend::collect`.
- `TraceControlLoop::run(receiver: Receiver<WatchSignal>, collection_sender: Sender<CollectionCommand>)` handles hooks/observer signals without collection calls.
- `start_live_collection(state, app) -> LiveCollectionHandle` owns one control thread and one collection thread; shutdown remains idempotent and joins both.

- [x] **Step 1: Write failing isolation and lifecycle scheduling tests.**

Add deterministic tests for:

```rust
#[test]
fn activate_arms_collection_and_reconciliation() {}

#[test]
fn final_flush_is_requested_after_stop_and_does_not_keep_reconciliation_armed() {}

#[test]
fn no_active_provider_means_no_due_collection_deadline() {}

#[test]
fn blocked_collection_does_not_block_trace_publication() {}

#[test]
fn final_flush_summary_cannot_reactivate_a_stopped_provider() {}
```

Use a scripted backend with a barrier for the blocked-collection case and a recording publisher. Do not sleep in deterministic scheduler tests.

- [x] **Step 2: Run the focused tests and confirm they fail.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib app::live_collection --offline
```

Expected: the current loop has no activation state, no final-flush reason, and handles Trace signals in the same worker as `collect`.

- [x] **Step 3: Implement lifecycle-aware collection commands and scheduler.**

Keep one `CollectionBackend` and one `SummaryPublisher`. Track active providers in the collection loop, arm reconciliation only when the set is non-empty, and make Stop schedule one immediate final flush. Preserve the existing 200ms notification coalescing and exponential storage retry. Treat publisher failure as post-commit emission failure, not a collection failure.

After a successful collection, publish `state.summary()` so the latest lifecycle snapshot is applied after commit. Never publish a raw report summary that can restore Active after Stop.

- [x] **Step 4: Implement the non-blocking control loop.**

Route `Trace` signals through `AppState::apply_trace_signal`, publish its immediate summary, and send only the returned lifecycle command to the collection worker. Route observer/configuration signals to collection commands. The control loop must not call `collect_once`, query raw files, or wait for the collection worker.

- [x] **Step 5: Run focused live-loop tests.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib app::live_collection --offline
```

Expected: isolation, activation, Stop/final-flush, retry, shutdown, and existing debounce tests pass.

- [x] **Step 6: Commit the split runtime.**

```powershell
git add src-tauri/src/app/live_collection.rs src-tauri/src/app/runtime.rs
git commit -m "feat: separate hook control from collection work"
```

## Task 3: Make the observer dynamic and provider-leased

**Files:**
- Modify: `src-tauri/src/sources/file_watcher.rs`
- Modify: `src-tauri/src/sources/mod.rs`
- Modify: `src-tauri/src/app/live_collection.rs`
- Test: `src-tauri/src/sources/file_watcher.rs` and live-loop tests

**Interfaces:**
- `SourceObserver::new(sender: Sender<WatchSignal>) -> SourceObserver` creates no worker.
- `SourceObserver::start_provider(root: WatchRoot)` starts one native observer for that provider.
- `SourceObserver::stop_provider(provider)` cancels and joins only that provider's workers.
- `SourceObserver::replace_roots(roots)` refreshes only currently leased providers.
- `SourceObserver::shutdown()` is idempotent.

- [x] **Step 1: Write failing dynamic observer tests.**

Add Windows tests that assert:

```rust
#[test]
fn observer_starts_only_after_provider_activation() {}

#[test]
fn stopping_one_provider_does_not_stop_the_other() {}

#[test]
fn stopping_provider_closes_native_workers() {}
```

Use temporary directories and synthetic metadata-only writes. Assert only provider signals are emitted and no filename appears in debug output.

- [x] **Step 2: Run the observer tests and confirm the current startup watcher fails the lifecycle expectation.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib sources::file_watcher --offline
```

- [x] **Step 3: Implement provider-scoped observer ownership.**

Rename the implementation type to `SourceObserver` while preserving the native Win32 FFI. Replace the single all-roots worker collection with a provider-keyed worker map. Remove the one-second readiness wait from the control path; startup failure is reported asynchronously as a provider-scoped signal. Keep cancellation through `CancelIoEx`, worker joins, bounded notification validation, and path-free signals.

- [x] **Step 4: Connect activation/Stop leases to observer start/stop.**

On the first active run for a provider, resolve its Rust-only `WatchRoot` and call `start_provider`. On the last matching Stop/expiry, call `stop_provider`. A second session for the same provider reuses the observer. Configuration refresh replaces roots only for currently active providers.

- [x] **Step 5: Run focused observer and live-loop tests.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib sources::file_watcher --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib app::live_collection --offline
```

Expected: dynamic start/stop, provider independence, cancellation, and path privacy pass.

- [x] **Step 6: Commit the observer slice.**

```powershell
git add src-tauri/src/sources/file_watcher.rs src-tauri/src/sources/mod.rs src-tauri/src/app/live_collection.rs
git commit -m "feat: lease provider observers to active runs"
```

## Task 4: Harden hook delivery and wire startup/shutdown

**Files:**
- Modify: `src-tauri/src/app/trace_signal.rs`
- Modify: `src-tauri/src/app/live_collection.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/app/trace_signal.rs`, `src-tauri/tests/trace_signal.rs`

**Interfaces:**
- Hook mode retains `run_hook_mode() -> bool` and fail-open exit behavior.
- Pipe send performs a small bounded retry window and never logs raw input.
- `HookListener` is owned by the control loop and is stopped before the observer and collection worker are joined.

- [x] **Step 1: Write failing pipe retry and startup ownership tests.**

Test the pure retry policy with a scripted attempt function:

```rust
#[test]
fn pipe_retry_stops_at_the_bounded_attempt_limit() {}

#[test]
fn hook_mode_still_exits_zero_when_the_app_is_not_running() {}

#[test]
fn shutdown_stops_listener_observer_and_collection_worker_idempotently() {}
```

Keep the Windows FFI test-free where the existing platform seam is not available; use the smallest pure helper seam for retry count/delay.

- [x] **Step 2: Run focused trace tests and confirm the retry seam is absent.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test trace_signal --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib app::trace_signal --offline
```

- [x] **Step 3: Implement bounded send retry.**

Retry only transient pipe-open/write failure for a short fixed budget, with no unbounded wait and no stdout/stderr. Keep the hook fail-open when the app is closed. Preserve payload-size, schema, event, timestamp, and opaque-ID validation.

- [x] **Step 4: Wire two-lane startup and shutdown.**

Keep the existing initial startup collection for restart recovery. Start the control/collection runtime after that initial attempt. Do not start `SourceObserver` until an activation signal. On `RunEvent::Exit`, stop the hook listener, dynamic observers, and collection worker in a deterministic idempotent order.

- [x] **Step 5: Run focused cross-boundary tests.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test trace_signal --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib app::trace_signal --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib app::live_collection --offline
```

- [x] **Step 6: Commit hook/runtime wiring.**

```powershell
git add src-tauri/src/app/trace_signal.rs src-tauri/src/app/live_collection.rs src-tauri/src/lib.rs
git commit -m "feat: wire event-driven hook runtime"
```

## Task 5: Regression, privacy, and integrated verification

**Files:**
- Review: all files changed in Tasks 1–4
- Test: existing Rust/frontend suites and focused new tests
- Verify unchanged: `src-tauri/src/database/schema.rs`, `src-tauri/src/types/usage_summary.rs`, frontend usage bridge

- [x] **Step 1: Run Rust formatting and focused tests.**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib app::runtime --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib app::live_collection --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib sources::file_watcher --offline
cargo test --manifest-path src-tauri/Cargo.toml --test trace_signal --offline
```

- [x] **Step 2: Run collection, database, provider, and privacy regressions.**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test collection_core --test database --test provider_readers --test session_summary --offline
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
```

Expected: no lifecycle signal appears in SQLite, no raw hook field appears in diagnostics/serialized payloads, and cumulative delta/dedup/restart behavior is unchanged.

- [x] **Step 3: Run frontend gates.**

```powershell
npm test -- --run
npm run build
```

Expected: the existing event-driven frontend bridge and UI tests pass without a new frontend contract.

- [x] **Step 4: Run the integrated debug build.**

```powershell
npm run tauri build -- --debug
```

Expected: one debug Tauri executable is produced, with no new sidecar, service, network client, or dependency.

- [ ] **Step 5: Perform Windows smoke checks.**

  Automated native observer and hook projection checks pass. Manual smoke with
  the trusted Claude/Codex processes remains a user-run acceptance step.

Verify with real trusted hooks:

1. No active run means no source observer activity.
2. `UserPromptSubmit` changes the widget to Active immediately.
3. A token record updates totals after commit.
4. `Stop` changes the widget to Idle immediately and stops the provider observer when it is the last run.
5. Two simultaneous sessions share one provider observer; stopping one leaves the other Active.
6. Closing and reopening restores SQLite totals; the next activation resumes live observation.
7. App-closed hook invocation exits zero and does not create a raw fallback file.

- [x] **Step 6: Review the final diff and commit the verification result.**

```powershell
git diff --check
git status --short --branch
git diff --name-only HEAD~4..HEAD
```

Confirm database schema and frontend files are unchanged, generated artifacts remain ignored, and only intended source/docs/tests are staged. Commit any final test-only or documentation correction separately with a responsibility-specific message.

Review completed: the default-target Tauri debug build, Rust suites, frontend
suites, privacy checks, and app-closed hook fail-open smoke all pass. The only
remaining acceptance step is the manual smoke with trusted Claude/Codex
processes in the running app.
