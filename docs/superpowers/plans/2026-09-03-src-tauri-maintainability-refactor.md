# src-tauri Maintainability Refactor Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Reduce accidental coupling and dead surface in src-tauri while keeping collection, privacy, persistence, runtime, and frontend contracts behavior-compatible.

**Architecture:** The collection core owns its collection-facing persistence seam and no longer imports the SQLite implementation. The SQLite store implements that seam, the Usage Summary calculation becomes one deep module with private helpers, and live collection becomes a small public module backed by private scheduler/controller/Adapter modules. Current token fields remain intact; this pass prepares locality for future metric expansion without adding a new metric to the UI.

**Tech Stack:** Rust 2021, Tauri 2, rusqlite with bundled SQLite, serde, native Windows APIs, Cargo tests and Clippy.

**Spec:** docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md

## Global Constraints

- Keep version one Windows 11-only, local-only, and metadata-only.
- Rust owns filesystem access, source discovery, collection, and SQLite access.
- React receives typed normalized summaries and configured source roots only in settings flows.
- Never store or emit prompts, responses, reasoning, tool payloads, credentials, repository contents, working directories, raw provider records, or arbitrary file contents.
- Preserve ProviderAdapter, path-free observer signals, reconciliation, restart-safe checkpoints, cumulative-to-delta conversion, deduplication, independent Provider failures, and post-commit summary publication.
- Keep CollectionCoordinator::collect as the only collection path.
- Do not add a network client, telemetry, sidecar, background service, frontend state library, CSS framework, ORM, font package, or new Cargo dependency.
- Do not add user-facing input/output/cache metrics in this refactor; preserve existing metric fields for a later focused feature.
- Work on dev; do not push or merge to main during this plan.
- Existing user-owned diagram changes are outside scope and must remain untouched.

---

### Task 1: Remove dead surface and repair naming

**Files:**

- Delete: src-tauri/src/app/startup.rs
- Modify: src-tauri/src/app/mod.rs
- Modify: src-tauri/src/types/file_checkpoint.rs
- Modify: src-tauri/src/types/widget_settings.rs
- Modify: src-tauri/src/sources/session_files.rs
- Modify: src-tauri/src/sources/provider_roots.rs
- Modify: src-tauri/src/app/runtime.rs
- Modify: src-tauri/src/sources/file_watcher.rs
- Modify: src-tauri/src/commands/source_settings.rs
- Modify: src-tauri/src/app/live_collection.rs

**Interfaces:**

- No public behavior changes.
- Preserve ProviderRoot::configured_root, SourceConfig::configured_root_label, and DiscoveredSessionFile::filesystem_path.
- Remove only symbols with no production or test caller.

    - [x] Step 1: Re-run the deletion search.

    rg -n "\b(is_compatible_with|root_relative|WidgetSettings)\b" src-tauri/src src-tauri/tests
    rg -n "\b(startup|count_usage_events|discover_native_sources|discover_provider)\b" src-tauri/src src-tauri/tests

    Expected: the first command identifies the three confirmed dead symbols; the second distinguishes the empty startup module from test-only helpers.

    - [x] Step 2: Delete confirmed dead symbols.

    Remove the empty startup module declaration and file, FileCheckpoint::is_compatible_with, WidgetSettings, and DiscoveryResult::root_relative. Keep test-only helpers until their tests are moved or deleted in a later focused change.

    - [x] Step 3: Remove stale suppression and repair names/docs.

    Remove production-used allow(dead_code) attributes. Rename root accessors so configured labels and filesystem paths are explicit. Update stale runtime and watcher documentation without changing runtime behavior.

    - [x] Step 4: Run the narrow Rust gate.

    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo test --manifest-path src-tauri/Cargo.toml

    Expected: formatting passes and all existing tests pass.

    - [x] Step 5: Commit the focused cleanup.

    git add src-tauri/src/app src-tauri/src/types src-tauri/src/sources src-tauri/src/commands
    git commit -m "chore: remove stale src-tauri surface"

---

### Task 2: Make collection ownership one-way

**Files:**

- Create: src-tauri/src/collection/coordinator.rs
- Create: src-tauri/src/collection/source_collection.rs
- Create: src-tauri/src/collection/persistence.rs
- Modify: src-tauri/src/collection/mod.rs
- Create: src-tauri/src/usage/summary/mod.rs with SummaryRows
- Rename: src-tauri/src/database/connection.rs to src-tauri/src/database/store.rs
- Modify: src-tauri/src/database/mod.rs
- Modify: src-tauri/src/database/store.rs
- Modify: src-tauri/src/app/runtime.rs
- Modify: src-tauri/tests/collection_core.rs
- Modify: src-tauri/tests/database.rs
- Modify: src-tauri/tests/runtime_integration.rs

**Interfaces:**

- collection/persistence.rs owns CollectionBatch, collection update records, CollectionStore, and the collection-facing storage error.
- collection/source_collection.rs owns the private SourceCollectionResult returned by per-Source collection.
- usage/summary/mod.rs owns SummaryRows { events, rate_limits }.
- database/store.rs implements CollectionStore for IndexStore and maps SQLite failures into the collection-facing storage error.
- collection must not import database.
- CollectionCoordinator::collect remains the caller-visible collection entry point.

    - [x] Step 1: Add collection-facing persistence records.

    Move CollectionBatch, SessionKeyUpdate, SessionNameUpdate, RateLimitUpdate, SourceUpdate, and DiagnosticUpdate into collection/persistence.rs. Move CollectionStore there and replace the database-owned error in that contract with a collection-facing storage error.

    - [x] Step 2: Add the named Source result.

    Replace the nine-value tuple returned by collect_source with SourceCollectionResult. Keep existing validation, byte-budget, checkpoint, diagnostic, and Provider-independence behavior unchanged.

    - [x] Step 3: Move coordinator implementation.

    Move CollectionCoordinator, CollectionReport, CollectionError, and collection clock implementations into collection/coordinator.rs. Move the private per-Source method into collection/source_collection.rs. Re-export only existing caller-facing names from collection/mod.rs.

    - [x] Step 4: Move the summary input type.

    Define SummaryRows in usage/summary/mod.rs so collection and database use the normalized read model without collection importing database.

    - [x] Step 5: Make SQLite the Adapter.

    Rename connection.rs to store.rs. Implement CollectionStore for IndexStore in the database module. Keep SQL transaction order and atomic event/checkpoint/Source Health/diagnostic updates unchanged.

    - [x] Step 6: Keep settings persistence in the runtime seam.

    Expose only the smallest store access needed by app/runtime.rs for existing source-settings and widget-settings persistence. Remove the specialized CollectionCoordinator<IndexStore> implementation that forces collection to import database.

    - [x] Step 7: Move tests to the correct seam.

    Keep collection integration tests in tests/collection_core.rs, database transaction tests in tests/database.rs, and runtime persistence tests in tests/runtime_integration.rs. Update imports through re-exports; do not make SQL helpers public.

    - [x] Step 8: Run the collection/database gate.

    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo check --manifest-path src-tauri/Cargo.toml
    cargo test --manifest-path src-tauri/Cargo.toml

    Expected: collection, database, runtime, privacy, checkpoint, and Provider tests pass.

    - [x] Step 9: Commit the ownership change.

    git add src-tauri/src/collection src-tauri/src/usage/summary src-tauri/src/database src-tauri/src/app/runtime.rs src-tauri/tests
    git commit -m "refactor: isolate collection from sqlite ownership"

---

### Task 3: Deepen Usage Summary calculation

**Files:**

- Create: src-tauri/src/usage/summary/active_provider.rs
- Create: src-tauri/src/usage/summary/provider.rs
- Create: src-tauri/src/usage/summary/sessions.rs
- Create: src-tauri/src/usage/summary/daily_total.rs
- Modify: src-tauri/src/usage/summary/mod.rs
- Modify: src-tauri/src/usage/mod.rs
- Delete: src-tauri/src/usage/active_provider.rs
- Delete: src-tauri/src/usage/provider_summary.rs
- Delete: src-tauri/src/usage/session_summary.rs
- Delete: src-tauri/src/usage/daily_total.rs
- Modify: src-tauri/src/collection/coordinator.rs
- Modify: src-tauri/tests/collection_core.rs
- Modify: src-tauri/tests/provider_summary.rs
- Modify: src-tauri/tests/session_summary.rs

**Interfaces:**

- usage::summary::compute_summary is the single public Usage Summary calculation seam.
- Its inputs remain normalized SummaryRows, Source Health, enabled Providers, now, and the Windows local day.
- Summary helpers are private implementation modules.
- UsageSummary, ProviderUsageSummary, and SessionUsageSummary wire shapes remain unchanged.

    - [x] Step 1: Move the four calculation implementations.

    Move active-Provider selection, per-Provider calculation, Session aggregation, and local-day total calculation under usage/summary/. Preserve the 15-second activity window, future-event filtering, Session ordering, current-local-day behavior, and idle fallback.

    - [x] Step 2: Centralize summary assembly.

    Move compute_summary out of collection into usage::summary. Pass plain time values instead of the collection clock trait so the Usage Summary module has no collection dependency.

    - [x] Step 3: Remove repeated scan entry points.

    Make the summary module call private aggregation helpers from one orchestration path. Do not add a cache, stateful summary manager, or generic metric registry.

    - [x] Step 4: Relocate private tests.

    Move helper-specific tests into separate module test files where private access is required. Keep integration tests focused on the public Usage Summary result and serialized contract.

    - [x] Step 5: Run the Usage Summary gate.

    cargo test --manifest-path src-tauri/Cargo.toml usage
    cargo test --manifest-path src-tauri/Cargo.toml

    Expected: Session, Provider, local-day, active/idle, multi-Session, and wire-contract tests pass.

    - [x] Step 6: Commit the summary deepening.

    git add src-tauri/src/usage src-tauri/src/collection/coordinator.rs src-tauri/tests
    git commit -m "refactor: deepen usage summary calculation"

---

### Task 4: Simplify live collection ownership

**Files:**

- Create: src-tauri/src/app/live_collection/mod.rs
- Create: src-tauri/src/app/live_collection/scheduler.rs
- Create: src-tauri/src/app/live_collection/controller.rs
- Create: src-tauri/src/app/live_collection/adapters.rs
- Create: src-tauri/src/app/live_collection/scheduler_tests.rs
- Create: src-tauri/src/app/live_collection/controller_tests.rs
- Delete: src-tauri/src/app/live_collection.rs
- Modify: src-tauri/src/app/mod.rs
- Modify: src-tauri/tests/runtime_integration.rs

**Interfaces:**

- Preserve start_live_collection, LiveCollectionHandle, update_source_config_and_refresh, CollectionBackend, and SummaryPublisher.
- scheduler.rs owns deadline, debounce, retry, reconciliation, and activity-expiry policy.
- controller.rs owns SourceObserver, one worker loop, one observed-Provider set, shutdown, and live collection lifecycle.
- adapters.rs owns RuntimeBackend and TauriSummaryPublisher.
- CollectionCommand and LiveCollectionLoop are removed because WatchSignal is already the path-free control protocol.

    - [x] Step 1: Move scheduler policy and tests.

    Move LiveScheduler and CollectionReason with deterministic tests into scheduler.rs and scheduler_tests.rs. Keep the existing 200 ms debounce, 30-second reconciliation, activity expiry, and 1/2/4/8/16/30-second retry policy.

    - [x] Step 2: Move production Adapters.

    Move RuntimeBackend and TauriSummaryPublisher into adapters.rs. Keep existing error mapping and post-commit publication behavior.

    - [x] Step 3: Collapse the pass-through worker protocol.

    Make LiveCollectionController own the scheduler, backend, publisher, observer, and one observed_providers set. Handle WatchSignal directly and remove the second channel, CollectionCommand, and LiveCollectionLoop.

    - [x] Step 4: Preserve lifecycle behavior.

    Keep observer activation only for enabled existing roots, path-free signals, idempotent shutdown, worker joining, source refresh after settings persistence, and independent Provider watcher failure.

    - [x] Step 5: Split private tests.

    Move scheduler tests into scheduler_tests.rs and controller/lifecycle tests into controller_tests.rs. Keep private types private; do not widen visibility only to satisfy tests.

    - [x] Step 6: Run the live collection gate.

    cargo test --manifest-path src-tauri/Cargo.toml live_collection
    cargo test --manifest-path src-tauri/Cargo.toml

    Expected: debounce, retry, reconciliation, observer, refresh, shutdown, post-commit publication, and partial-write tests pass.

    - [x] Step 7: Commit the runtime simplification.

    git add src-tauri/src/app src-tauri/tests/runtime_integration.rs
    git commit -m "refactor: simplify live collection ownership"

---

### Task 5: Full verification and review gate

> Verification note: the native desktop Computer Use surface was unavailable
> in this session. The debug executable launch smoke (`--hook claude`) passed;
> full Rust lifecycle coverage and the packaged debug build also passed.

**Files:**

- Verify only files changed by Tasks 1–4.
- Do not modify diagrams, generated build output, browser-review artifacts, dependency directories, or local .claude/ settings.

    - [x] Step 1: Run all Rust checks.

    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo check --manifest-path src-tauri/Cargo.toml
    cargo test --manifest-path src-tauri/Cargo.toml
    cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -W clippy::all

    Expected: format, check, and tests pass. Clippy warnings must be reviewed; no warning may be silenced without a concrete invariant or platform reason.

    - [x] Step 2: Run the Tauri package build.

    npm run tauri build -- --debug

    Expected: the debug Windows package builds successfully and the executable is produced under src-tauri/target/debug/.

- [ ] Step 3: Perform Windows smoke checks.

    Verify startup, observer activation, appended-record refresh, idle expiry, independent Provider failure, settings refresh, tray visibility, shutdown, frameless window behavior, and absence of raw source data in emitted summaries.

    - [x] Step 4: Inspect the final diff.

    git diff --check
    git status --short
    git diff --stat

    Expected: only intended Rust source, tests, and this plan are changed; existing diagram work remains untouched.

---

## Explicitly deferred

The current pipeline already carries input, cached-input, output, and total token fields through observations, delta conversion, checkpoints, Usage Events, and SQLite. This refactor will not add a TokenMetrics abstraction or new UI fields preemptively. When the first new metric is requested, introduce a typed metric value object at that feature's seam and add aggregation, storage, and UI contract tests in a separate focused change.
