# Runtime Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder Tauri startup summary with one real, restart-safe native collection pass and a typed `UsageSummary` command/event seam consumed by React.

**Architecture:** A Rust `AppState` owns a `CollectionCoordinator<IndexStore>`, the fixed current-user profile root, and explicit discovery limits behind a Tauri-managed mutex. Tauri setup performs one synchronous collection pass; only a summary from a successful post-commit report is emitted through the `usage-summary-changed` event. React subscribes before requesting the initial summary and accepts only the allow-listed wire shape. File watching and shell lifecycle features remain separate slices.

**Tech Stack:** Rust 2021, Tauri 2, `rusqlite`, `serde`, React 19, TypeScript, Vite, Vitest, and the existing Tauri 2 JavaScript API.

**Spec:** `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`

## Global Constraints

- Version 1 remains local-only and Windows 11-only.
- Tauri 2 is the desktop shell and Rust owns filesystem and SQLite access.
- SQLite is accessed only by the Rust core; React receives no database handle or raw observation.
- Adapters emit normalized metadata only: provider, opaque session/event keys, timestamp, counter kind, token counters.
- Prompts, responses, reasoning, tool payloads, credentials, repository contents, raw JSON, and working directories never enter normalized events, SQLite, diagnostics, or frontend payloads.
- Incremental observations are stored once; cumulative observations are ordered and converted to deltas; cumulative values are never summed directly.
- A cumulative decrease starts a new monotonic segment and never creates a negative delta.
- Stable event identities survive restart, rescan, truncation, and file rotation.
- Claude and Codex source health stays independent.
- No network client, telemetry, sidecar, background service, frontend state library, CSS framework, ORM, or WSL auto-discovery is added.
- The React webview receives typed summaries only; it cannot request raw observations or arbitrary files.
- The runtime uses only the native roots already implemented by source discovery: `%USERPROFILE%\.claude\projects` and `%USERPROFILE%\.codex\sessions`.
- No code in this plan uses `src-tauri/src/sources/file_watcher.rs`, `src-tauri/src/app/tray.rs`, `src-tauri/src/app/startup.rs`, or `src-tauri/src/app/window.rs`.

## Scope Boundary

This slice makes the existing tested Rust collection core reachable from the shipped Tauri executable and makes the existing React shell display its committed summary. It includes a single startup collection pass and the live-summary protocol seam that a later watcher can call.

It does not implement the watcher or 30-second reconciliation, tray actions, settings, explicit WSL UNC roots, single-instance enforcement, launch-on-login, remembered window position, opacity controls, installer/uninstaller behavior, or the later sizing/overflow polish pass.

## Baseline Evidence

- `dev` is clean at `889f7ba` (`docs: add collection implementation plans`).
- The Rust collection, provider-reader, source-discovery, database, and probe tests currently pass: 73 tests across the Rust targets.
- `src-tauri/src/lib.rs::get_usage_summary` still returns `UsageSummary::loading()` and `run()` does not manage a runtime state.
- `src-tauri/src/main.rs` has no release-only Windows GUI subsystem attribute, so the release executable opens a console window.
- `src-tauri/src/app/startup.rs`, `tray.rs`, `window.rs`, `src-tauri/src/sources/file_watcher.rs`, and `src-tauri/src/commands/usage_summary.rs` are still scaffolds.
- Frontend commands are expected to require an approved/elevated execution context in this environment because bundled esbuild currently fails with `spawn EPERM`; do not change application code to work around that environment restriction.

## File Map

- Create `src-tauri/src/app/runtime.rs` for the deep runtime seam: managed state, production path resolution, bounded native discovery, adapter construction, and one collection pass.
- Modify `src-tauri/src/app/mod.rs` to expose the runtime module.
- Modify `src-tauri/src/commands/usage_summary.rs` to implement the existing summary command and define the one typed summary event.
- Review `src-tauri/src/commands/mod.rs`; its existing `pub mod usage_summary;` declaration is sufficient and should remain unchanged.
- Modify `src-tauri/src/lib.rs` to manage `AppState`, run the first collection during Tauri setup, emit only a post-commit summary, and register the command.
- Modify `src-tauri/src/main.rs` to suppress the release console window with the standard Windows subsystem attribute.
- Modify `src-tauri/src/types/usage_summary.rs`, `src-tauri/src/types/provider.rs`, and `src-tauri/src/usage/active_provider.rs` for unavailable fallback construction and user-facing provider labels.
- Create `src-tauri/tests/runtime_integration.rs` for synthetic-profile runtime, restart, provider-independence, and privacy tests.
- Modify `src/App.tsx` and `src/lib/usage-summary.ts` for event subscription, initial command loading, runtime payload validation, and the relative-update footer.
- Modify `src/App.test.tsx` and `src/lib/usage-summary.test.ts` for event updates, cleanup, invalid-payload rejection, and relative-time formatting.
- Do not modify provider parsers, database schema, `tauri.conf.json`, capabilities, or package dependencies unless a focused gate proves an integration regression in an already-listed file.

---

### Task 1: Remove the release console window

**Files:**
- Modify: `src-tauri/src/main.rs`
- Verify: `src-tauri/target/release/token-tracing-widget.exe`

**Interfaces:**
- Produces the same `main()` entry point with release builds marked as the Windows GUI subsystem; debug builds keep their existing console behavior for development diagnostics.

- [ ] **Step 1: Capture the current release behavior before editing.**

Run:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --release --offline
```

Launch `src-tauri/target/release/token-tracing-widget.exe` from Explorer and record that the current release artifact opens a console window, matching the handoff. Stop the process before continuing.

- [ ] **Step 2: Add the release-only subsystem attribute.**

Put this crate attribute before `fn main()` in `src-tauri/src/main.rs` and leave the entry point unchanged:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    token_tracing_widget_lib::run();
}
```

- [ ] **Step 3: Rebuild and verify the PE subsystem.**

Run:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --release --offline
dumpbin /headers src-tauri/target/release/token-tracing-widget.exe | Select-String -Pattern "subsystem"
```

Expected: the PE header reports `Windows GUI`; starting the release executable from Explorer produces no console window. The debug build remains usable from a terminal.

- [ ] **Step 4: Run the narrow regression checks.**

Run:

```powershell
git diff --check
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline
```

Expected: no whitespace errors and the existing Rust library test passes.

- [ ] **Step 5: Commit the isolated shell fix.**

```text
git add src-tauri/src/main.rs
git commit -m "fix: suppress release console window"
```

### Task 2: Add the Rust runtime state and native collection seam

**Files:**
- Create: `src-tauri/src/app/runtime.rs`
- Modify: `src-tauri/src/app/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/types/usage_summary.rs`
- Modify: `src-tauri/src/types/provider.rs`
- Modify: `src-tauri/src/usage/active_provider.rs`
- Create: `src-tauri/tests/runtime_integration.rs`

**Interfaces:**
- `DEFAULT_DISCOVERY_LIMITS: DiscoveryLimits` is `5` files and `50 * 1024 * 1024` bytes.
- `RuntimeInitError` has only `DataDirectory`, `DatabaseOpen`, and `ProfileUnavailable` variants, displayed as `data_directory`, `database_open`, and `profile_unavailable`; `RuntimeError` has only `Unavailable`, `StatePoisoned`, and `Collection(CollectionError)` variants, displayed as `unavailable`, `state_poisoned`, and `collection:<category>`.
- `AppState::from_paths(profile_root: PathBuf, database_path: &Path, limits: DiscoveryLimits) -> Result<AppState, RuntimeInitError>` creates the local SQLite-backed runtime for tests and future shell callers.
- `AppState::unavailable() -> AppState` creates a safe fallback state when the profile or application-data path cannot be resolved.
- `AppState::collect_once(&self, clock: &dyn CollectionClock) -> Result<CollectionReport, RuntimeError>` discovers both native providers, constructs `ClaudeReader` and `CodexReader`, and delegates to the existing coordinator.
- `AppState::summary(&self) -> UsageSummary` returns the coordinator's last committed summary, or the sanitized unavailable fallback if no runtime exists or the state mutex is poisoned.
- `initialize_from_app(app: &tauri::AppHandle) -> AppState` reads only the current user's `USERPROFILE` and Tauri's local application-data directory; it maps all setup failures to path-free categories.
- `Provider::display_name() -> &'static str` returns `Claude Code` or `Codex` for the summary header while `Provider::as_str()` remains the stable lowercase storage identifier.
- `UsageSummary::unavailable() -> UsageSummary` returns `Unavailable`, zero today's total, no provider, no current-session total, no last-update timestamp, and an empty health list.
- The crate root re-exports `AppState` so integration tests use `token_tracing_widget_lib::AppState` without making the whole `app` module public.

- [ ] **Step 1: Write the failing runtime integration tests.**

Create `src-tauri/tests/runtime_integration.rs` with synthetic records only. The helper must write exactly these token-bearing shapes beneath the already-fixed native roots and must not create prompts, responses, repository paths, or other provider fields:

```rust
use std::fs;

use tempfile::TempDir;
use token_tracing_widget_lib::AppState;
use token_tracing_widget_lib::collection::FixedClock;
use token_tracing_widget_lib::sources::session_files::DiscoveryLimits;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::UsageState;

fn write_profile(include_codex: bool) -> TempDir {
    let profile = tempfile::tempdir().expect("profile should be created");
    let claude_root = profile.path().join(r".claude\projects");
    fs::create_dir_all(&claude_root).expect("Claude root should be created");
    fs::write(
        claude_root.join("session.jsonl"),
        br#"{"message":{"id":"event-synthetic-001","type":"message","usage":{"input_tokens":10,"output_tokens":10}},"sessionId":"session-synthetic-001","timestamp":"2026-01-01T00:00:00Z"}
"#,
    )
    .expect("Claude fixture should be written");

    if include_codex {
        let codex_root = profile.path().join(r".codex\sessions");
        fs::create_dir_all(&codex_root).expect("Codex root should be created");
        fs::write(
            codex_root.join("session.jsonl"),
            br#"{"payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":10,"total_tokens":20}}},"timestamp":"2026-01-01T00:00:01Z"}
"#,
        )
        .expect("Codex fixture should be written");
    }

    profile
}

fn limits() -> DiscoveryLimits {
    DiscoveryLimits::new(10, 10_000)
}

#[test]
fn runtime_collects_native_sources_and_returns_post_commit_summary() {
    let profile = write_profile(true);
    let database = tempfile::tempdir().expect("database directory should be created");
    let state = AppState::from_paths(
        profile.path().to_path_buf(),
        &database.path().join(r"nested\index.sqlite"),
        limits(),
    )
    .expect("runtime should open");

    let report = state
        .collect_once(&FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"))
        .expect("initial collection should commit");

    assert_eq!(report.summary.today_tokens, 40);
    assert_eq!(report.summary.provider.as_deref(), Some("Codex"));
    assert_eq!(report.summary.state, UsageState::Active);
    assert_eq!(report.accepted_event_count, 2);
}

#[test]
fn runtime_restart_reuses_checkpoints_and_deduplicates_existing_events() {
    let profile = write_profile(true);
    let database = tempfile::tempdir().expect("database directory should be created");
    let database_path = database.path().join("index.sqlite");
    let clock = FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01");

    let first = AppState::from_paths(profile.path().to_path_buf(), &database_path, limits())
        .expect("first runtime should open");
    assert_eq!(first.collect_once(&clock).unwrap().summary.today_tokens, 40);
    drop(first);

    let second = AppState::from_paths(profile.path().to_path_buf(), &database_path, limits())
        .expect("restarted runtime should open");
    let report = second.collect_once(&clock).expect("restart should collect");

    assert_eq!(report.summary.today_tokens, 40);
    assert_eq!(report.accepted_event_count, 0);
}

#[test]
fn missing_codex_root_does_not_block_claude_collection() {
    let profile = write_profile(false);
    let database = tempfile::tempdir().expect("database directory should be created");
    let state = AppState::from_paths(profile.path().to_path_buf(), &database.path().join("index.sqlite"), limits())
        .expect("runtime should open");

    let report = state
        .collect_once(&FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01"))
        .expect("Claude collection should succeed");

    assert_eq!(report.summary.today_tokens, 20);
    assert_eq!(report.source_health.len(), 2);
    assert_eq!(report.source_health[0].provider, Provider::Claude);
    assert_eq!(report.source_health[0].state, "detected");
    assert_eq!(report.source_health[1].provider, Provider::Codex);
    assert_eq!(report.source_health[1].state, "not_detected");
}

#[test]
fn unavailable_fallback_contains_no_private_fields() {
    let serialized = serde_json::to_value(AppState::unavailable().summary()).unwrap();
    let object = serialized.as_object().unwrap();

    assert_eq!(object.get("state").and_then(|value| value.as_str()), Some("unavailable"));
    assert_eq!(object.get("todayTokens").and_then(|value| value.as_u64()), Some(0));
    assert!(!object.contains_key("profileRoot"));
    assert!(!object.contains_key("databasePath"));
    assert!(!object.contains_key("rawRecord"));
}
```

- [ ] **Step 2: Run the focused tests to verify the seam is absent.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test runtime_integration --offline
```

Expected: integration-test compilation fails because the re-exported `AppState` and its methods do not exist yet.

- [ ] **Step 3: Implement the private runtime implementation behind the small `AppState` interface.**

Keep the profile path and `CollectionCoordinator<IndexStore>` inside Rust. The implementation shape is:

```rust
struct Runtime {
    coordinator: CollectionCoordinator<IndexStore>,
    profile_root: PathBuf,
    discovery_limits: DiscoveryLimits,
}

pub struct AppState {
    runtime: Mutex<Option<Runtime>>,
    fallback_summary: UsageSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeInitError {
    ProfileUnavailable,
    DataDirectory,
    DatabaseOpen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Unavailable,
    StatePoisoned,
    Collection(CollectionError),
}

impl Runtime {
    fn collect_once(
        &mut self,
        clock: &dyn CollectionClock,
    ) -> Result<CollectionReport, CollectionError> {
        let [claude_discovery, codex_discovery] =
            discover_native_sources(&self.profile_root, self.discovery_limits);
        let claude_reader = ClaudeReader::default();
        let codex_reader = CodexReader::default();
        let sources = [
            ProviderSource::new(true, claude_discovery, &claude_reader),
            ProviderSource::new(true, codex_discovery, &codex_reader),
        ];
        self.coordinator.collect(&sources, clock)
    }
}
```

`AppState::from_paths` must create the database parent directory before calling `IndexStore::open`; map directory and database failures to payload-free `RuntimeInitError` variants. Re-export `AppState` from `src-tauri/src/lib.rs` while keeping `app` private. `AppState::collect_once` locks the runtime, returns `RuntimeError::Unavailable` when the option is empty, and never returns a path or observation. `AppState::summary` clones `Runtime::coordinator.last_summary()` and falls back to `fallback_summary` on an unavailable or poisoned state.

- [ ] **Step 4: Add production path resolution without introducing an arbitrary-root seam.**

Implement `initialize_from_app` with these exact inputs and fallbacks:

```rust
let profile_root = std::env::var_os("USERPROFILE")
    .filter(|value| !value.is_empty())
    .map(PathBuf::from);
let database_path = app
    .path()
    .app_local_data_dir()
    .ok()
    .map(|directory| directory.join("index.sqlite"));
```

If either value is absent, return `AppState::unavailable()`. Otherwise call `AppState::from_paths(profile_root, &database_path, DEFAULT_DISCOVERY_LIMITS)`. Do not call `wsl.exe`, enumerate distributions, read arbitrary sibling directories, or include either path in an error string.

- [ ] **Step 5: Align the summary provider label and fallback constructor with the wire contract.**

Add:

```rust
impl Provider {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
        }
    }
}
```

Use `latest.provider.display_name().to_owned()` only in `compute_active_provider`; keep database values from `as_str()`. Add `UsageSummary::unavailable()` beside `loading()` and `stale_from()`.

- [ ] **Step 6: Run the runtime tests and the existing Rust suite.**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --test runtime_integration --offline
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
```

Expected: the four new runtime tests and all existing provider, source, collection, database, and probe tests pass.

- [ ] **Step 7: Commit the runtime seam.**

```text
git add src-tauri/src/app/runtime.rs src-tauri/src/app/mod.rs src-tauri/src/types/usage_summary.rs src-tauri/src/types/provider.rs src-tauri/src/usage/active_provider.rs src-tauri/tests/runtime_integration.rs
git commit -m "feat: bootstrap native collection runtime"
```

### Task 3: Wire Tauri setup to the typed summary command and event

**Files:**
- Modify: `src-tauri/src/commands/usage_summary.rs`
- Review: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/usage_summary.rs`

**Interfaces:**
- `USAGE_SUMMARY_CHANGED_EVENT: &str` is exactly `"usage-summary-changed"`.
- `get_usage_summary(state: State<'_, AppState>) -> UsageSummary` remains the existing command name and returns only the last committed/fallback summary.
- `emit_usage_summary(app: &AppHandle, summary: &UsageSummary) -> Result<(), SummaryEventError>` emits the typed `UsageSummary` payload under `USAGE_SUMMARY_CHANGED_EVENT`.
- `SummaryEventError` contains only the category `Emit`; its display value is `emit` and never includes a Tauri error body.
- Tauri setup manages `AppState`, performs one `WindowsClock::current()` collection, and calls `emit_usage_summary` only when `collect_once` returns `Ok(CollectionReport)`.

- [ ] **Step 1: Write the failing command-contract tests.**

Add tests in `src-tauri/src/commands/usage_summary.rs` for the pure read helper and serialized allow-list:

```rust
#[test]
fn summary_contract_contains_only_allowed_wire_fields() {
    let summary = UsageSummary {
        state: UsageState::Active,
        provider: Some("Claude Code".to_owned()),
        current_session_tokens: Some(20),
        today_tokens: 20,
        last_updated_at: Some("2026-01-01T00:00:00Z".to_owned()),
        source_health: vec![SourceHealth::detected(Provider::Claude)],
    };
    let object = serde_json::to_value(summary)
        .expect("summary should serialize")
        .as_object()
        .cloned()
        .expect("summary should be an object");

    assert_eq!(
        object.keys().map(String::as_str).collect::<Vec<_>>(),
        vec![
            "currentSessionTokens",
            "lastUpdatedAt",
            "provider",
            "sourceHealth",
            "state",
            "todayTokens",
        ]
    );
    assert!(!object.contains_key("profileRoot"));
    assert!(!object.contains_key("rawRecord"));
}

#[test]
fn event_name_is_stable() {
    assert_eq!(USAGE_SUMMARY_CHANGED_EVENT, "usage-summary-changed");
}
```

- [ ] **Step 2: Run the focused command tests and verify the command seam is absent.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib commands::usage_summary --offline
```

Expected: compilation fails because the command function, event constant, and summary helper are not implemented.

- [ ] **Step 3: Implement the command and path-free event adapter.**

Use the existing Tauri command name and the managed state:

```rust
use tauri::{AppHandle, Emitter, State};

pub const USAGE_SUMMARY_CHANGED_EVENT: &str = "usage-summary-changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryEventError {
    Emit,
}

#[tauri::command]
pub fn get_usage_summary(state: State<'_, AppState>) -> UsageSummary {
    state.summary()
}

pub fn emit_usage_summary(
    app: &AppHandle,
    summary: &UsageSummary,
) -> Result<(), SummaryEventError> {
    app.emit(USAGE_SUMMARY_CHANGED_EVENT, summary)
        .map_err(|_| SummaryEventError::Emit)
}
```

Implement `Display` and `std::error::Error` for `SummaryEventError` with the only display value `emit`. Do not return `CollectionReport`, `SourceHealth` internals beyond its existing serialized fields, file paths, observations, or database errors from the command. Keep `SummaryEventError` path-free.

- [ ] **Step 4: Replace the loading-only builder setup.**

Refactor `src-tauri/src/lib.rs` so the builder follows this sequence:

```rust
use tauri::Manager;

tauri::Builder::default()
    .setup(|app| {
        let state = app::runtime::initialize_from_app(app.handle());
        app.manage(state);

        let managed = app.state::<app::runtime::AppState>();
        if let Ok(report) = managed.collect_once(&collection::WindowsClock::current()) {
            if commands::usage_summary::emit_usage_summary(app.handle(), &report.summary).is_err() {
                eprintln!("summary_event:emit");
            }
        }
        Ok(())
    })
    .invoke_handler(tauri::generate_handler![commands::usage_summary::get_usage_summary])
    .run(tauri::generate_context!())
    .expect("error while running token tracing widget");
```

The initial event may occur before React subscribes; the command immediately fetches the same managed summary, so startup does not depend on event timing. A failed collection leaves the coordinator's `Stale` summary available through the command and emits no uncommitted data. An initialization failure leaves an `Unavailable` fallback and keeps the application alive.

- [ ] **Step 5: Move the bootstrap serialization assertion to the new command/state contract.**

Remove the direct no-argument call to the old private `get_usage_summary()` from `src-tauri/src/lib.rs` tests. Keep an equivalent assertion against `UsageSummary::unavailable()` or `AppState::unavailable().summary()` and assert the same camelCase field allow-list.

- [ ] **Step 6: Run Rust command, runtime, and integrated build checks.**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
cargo check --manifest-path src-tauri/Cargo.toml --offline
```

Expected: command registration, managed-state types, event emission, and all Rust tests compile and pass without adding a dependency.

- [ ] **Step 7: Commit the Tauri boundary.**

```text
git add src-tauri/src/commands/usage_summary.rs src-tauri/src/lib.rs
git commit -m "feat: expose committed usage summary to tauri"
```

### Task 4: Subscribe the React overlay to validated summaries

**Files:**
- Modify: `src/lib/usage-summary.ts`
- Modify: `src/App.tsx`
- Modify: `src/lib/usage-summary.test.ts`
- Modify: `src/App.test.tsx`

**Interfaces:**
- `USAGE_SUMMARY_CHANGED_EVENT` is the same literal `"usage-summary-changed"` used by Rust.
- `parseUsageSummary(value: unknown) -> UsageSummary | null` rejects unknown top-level keys, unknown `sourceHealth` keys, invalid states, negative/non-integer counters, invalid optional fields, and non-array health values.
- `getUsageSummary() -> Promise<UsageSummary>` invokes `get_usage_summary`, validates the result with `parseUsageSummary`, and rejects with `Error("invalid_usage_summary")` if the allow-list fails.
- `listenForUsageSummary(onSummary: (summary: UsageSummary) => void) -> Promise<UnlistenFn>` listens to the one event, validates every payload, and ignores invalid payloads.
- `formatRelativeUpdate(lastUpdatedAt?: string, nowMs?: number) -> string` returns `No updates yet`, `Updated just now`, `Updated N min ago`, `Updated N hr ago`, or `Updated N d ago` using a non-negative elapsed duration.

- [ ] **Step 1: Write failing frontend tests for parsing, events, and relative time.**

Extend the existing mocks to cover `@tauri-apps/api/event` and add these cases:

```typescript
it("rejects a summary carrying a forbidden raw field", () => {
  expect(
    parseUsageSummary({
      state: "active",
      todayTokens: 20,
      sourceHealth: [],
      prompt: "private text",
    }),
  ).toBeNull();
});

it("forwards only valid summary-changed payloads", async () => {
  const onSummary = vi.fn();
  const stop = vi.fn();
  let emit: ((payload: unknown) => void) | undefined;
  vi.mocked(listen).mockImplementation(async (_event, handler) => {
    emit = (payload) => handler({ payload } as Parameters<typeof handler>[0]);
    return stop;
  });

  await listenForUsageSummary(onSummary);
  emit!({ state: "active", todayTokens: 20, sourceHealth: [] });
  emit!({ state: "active", todayTokens: 20, sourceHealth: [], rawRecord: "secret" });

  expect(onSummary).toHaveBeenCalledTimes(1);
  expect(onSummary).toHaveBeenCalledWith({ state: "active", todayTokens: 20, sourceHealth: [] });
});

it("formats relative update time without a polling timer", () => {
  const now = Date.parse("2026-01-01T00:10:00Z");

  expect(formatRelativeUpdate(undefined, now)).toBe("No updates yet");
  expect(formatRelativeUpdate("2026-01-01T00:09:30Z", now)).toBe("Updated just now");
  expect(formatRelativeUpdate("2026-01-01T00:05:00Z", now)).toBe("Updated 5 min ago");
});
```

Keep the test's event mock typed through the existing Vitest mock; it must not print or store the forbidden value.

- [ ] **Step 2: Run the focused frontend tests and verify the new helpers are absent.**

Run:

```powershell
npm test -- --run src/lib/usage-summary.test.ts
```

Expected: the test fails to compile or reports missing `parseUsageSummary`, `listenForUsageSummary`, and `formatRelativeUpdate`. If the sandbox returns esbuild `spawn EPERM`, rerun the same command only in the approved elevated context described by the handoff.

- [ ] **Step 3: Implement the allow-listed summary decoder and event adapter.**

Use a small decoder in `src/lib/usage-summary.ts` with these allowed keys:

```typescript
const summaryKeys = [
  "state",
  "provider",
  "currentSessionTokens",
  "todayTokens",
  "lastUpdatedAt",
  "sourceHealth",
] as const;

const sourceHealthKeys = ["provider", "state"] as const;
const states = new Set<UsageState>([
  "loading",
  "active",
  "idle",
  "unavailable",
  "stale",
]);
```

Require every object key to be in its corresponding allow-list. Require token fields to be finite non-negative safe integers, `state` to be one of `states`, optional provider/last-update values to be strings, and every health entry to contain only string `provider` and `state`. Use `invoke<unknown>` and `listen<unknown>` so the runtime decoder, rather than a TypeScript assertion, is the privacy gate.

The decoder and relative-time helper should have this concrete shape:

```typescript
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasOnlyKeys(value: Record<string, unknown>, allowed: readonly string[]): boolean {
  return Object.keys(value).every((key) => allowed.some((name) => name === key));
}

function isTokenCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

export function parseUsageSummary(value: unknown): UsageSummary | null {
  if (!isRecord(value) || !hasOnlyKeys(value, summaryKeys)) return null;
  if (typeof value.state !== "string" || !states.has(value.state as UsageState)) return null;
  const todayTokens = value.todayTokens;
  if (!isTokenCount(todayTokens)) return null;
  if ("provider" in value && typeof value.provider !== "string") return null;
  if ("currentSessionTokens" in value && !isTokenCount(value.currentSessionTokens)) return null;
  if (
    "lastUpdatedAt" in value &&
    (typeof value.lastUpdatedAt !== "string" || Number.isNaN(Date.parse(value.lastUpdatedAt)))
  ) return null;
  if (!Array.isArray(value.sourceHealth)) return null;

  const sourceHealth: SourceHealth[] = [];
  for (const entry of value.sourceHealth) {
    if (
      !isRecord(entry) ||
      !hasOnlyKeys(entry, sourceHealthKeys) ||
      typeof entry.provider !== "string" ||
      typeof entry.state !== "string"
    ) return null;
    sourceHealth.push({ provider: entry.provider, state: entry.state });
  }

  return {
    state: value.state as UsageState,
    ...(typeof value.provider === "string" ? { provider: value.provider } : {}),
    ...(isTokenCount(value.currentSessionTokens)
      ? { currentSessionTokens: value.currentSessionTokens }
      : {}),
    todayTokens,
    ...(typeof value.lastUpdatedAt === "string" ? { lastUpdatedAt: value.lastUpdatedAt } : {}),
    sourceHealth,
  };
}

export function formatRelativeUpdate(lastUpdatedAt?: string, nowMs = Date.now()): string {
  if (!lastUpdatedAt) return "No updates yet";
  const timestampMs = Date.parse(lastUpdatedAt);
  if (Number.isNaN(timestampMs)) return "No updates yet";
  const elapsedMs = Math.max(0, nowMs - timestampMs);
  if (elapsedMs < 60_000) return "Updated just now";
  const minutes = Math.floor(elapsedMs / 60_000);
  if (minutes < 60) return `Updated ${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `Updated ${hours} hr ago`;
  return `Updated ${Math.floor(hours / 24)} d ago`;
}
```

- [ ] **Step 4: Subscribe before the initial command and clean up on unmount.**

Update the `App` effect to register the listener first, then invoke the initial command. Preserve the existing mounted guard and add cleanup for the returned `UnlistenFn`:

```typescript
useEffect(() => {
  let mounted = true;
  let unlisten: UnlistenFn | undefined;

  const connect = async () => {
    try {
      const stop = await listenForUsageSummary((nextSummary) => {
        if (mounted) setSummary(nextSummary);
      });
      if (!mounted) {
        void stop();
        return;
      }
      unlisten = stop;
    } catch {
      if (mounted) setSummary(unavailableSummary);
    }

    try {
      const initialSummary = await getUsageSummary();
      if (mounted) setSummary(initialSummary);
    } catch {
      if (mounted) setSummary(unavailableSummary);
    }
  };

  void connect();
  return () => {
    mounted = false;
    if (unlisten) void unlisten();
  };
}, []);
```

Define `unavailableSummary` once beside the existing loading summary. Remove the bootstrap-only note and render `formatRelativeUpdate(summary.lastUpdatedAt)` in the footer. Keep the existing current-session fallback text and token formatting.

- [ ] **Step 5: Add UI regression tests for event updates and cleanup.**

Mock `listenForUsageSummary` to capture its callback and return a spy unlisten function. Render `App`, await the initial `Claude Code` summary, invoke the captured callback with a valid Codex summary, and assert that the header, current-session total, and today's total update. Unmount and assert the unlisten spy was called. Add one test that an initial invalid command payload drives the existing unavailable UI.

- [ ] **Step 6: Run frontend tests and the TypeScript/Vite build.**

Run:

```powershell
npm test -- --run
npm run build
```

Expected: all frontend tests pass, TypeScript compiles, and Vite builds without adding a state library, CSS framework, timer, polling loop, or network call.

- [ ] **Step 7: Commit the React boundary.**

```text
git add src/lib/usage-summary.ts src/App.tsx src/lib/usage-summary.test.ts src/App.test.tsx
git commit -m "feat: subscribe overlay to usage summaries"
```

### Task 5: Run end-to-end integration and standalone smoke gates

**Files:**
- Review: `src-tauri/src/app/runtime.rs`
- Review: `src-tauri/src/commands/usage_summary.rs`
- Review: `src-tauri/src/lib.rs`
- Review: `src-tauri/src/main.rs`
- Review: `src-tauri/src/types/usage_summary.rs`
- Review: `src-tauri/src/usage/active_provider.rs`
- Review: `src/App.tsx`
- Review: `src/lib/usage-summary.ts`
- Verify: `src-tauri/tests/runtime_integration.rs`

**Interfaces:**
- The shipped executable is one Tauri process with no app-managed sidecar or service.
- The only frontend data path is `get_usage_summary` plus `usage-summary-changed`, both carrying the `UsageSummary` allow-list.
- A provider failure is represented in its `SourceHealth` entry and does not suppress the other provider's result.
- A failed SQLite commit leaves the last summary stale and causes no summary event for uncommitted data.

- [ ] **Step 1: Run the complete Rust gate.**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
cargo check --manifest-path src-tauri/Cargo.toml --offline
```

Expected: all existing 73 Rust tests plus the runtime and command tests pass.

- [ ] **Step 2: Run the complete frontend gate.**

Run:

```powershell
npm test -- --run
npm run build
```

Expected: Vitest and Vite succeed. If the known esbuild `spawn EPERM` occurs, rerun these exact commands in the approved elevated context and record the result; do not loosen the app's security or add a workaround dependency.

- [ ] **Step 3: Build the integrated release executable.**

Run:

```powershell
npm run tauri build -- --no-bundle
```

Expected: `src-tauri/target/release/token-tracing-widget.exe` exists, contains the Windows GUI subsystem, and has no app-managed sidecar.

- [ ] **Step 4: Perform the standalone smoke check without exposing source content.**

Start `src-tauri/target/release/token-tracing-widget.exe` from Explorer and verify:

- no console window appears;
- the overlay no longer says `Bootstrap shell; collection is not enabled yet`;
- when a native source exists, the header shows `Claude Code` or `Codex`, current-session tokens, today's total, and a relative update footer;
- when a provider is absent, its health is represented without preventing the other provider from displaying;
- when no supported source exists, the overlay remains alive with `Unavailable`/`Not detected` state and zero today's tokens;
- no prompt, response, reasoning, tool payload, credential, repository content, working directory, raw JSON, or absolute path is printed or displayed.

Stop the executable after the smoke check.

- [ ] **Step 5: Recheck the privacy and scope boundary.**

Review the serialized `UsageSummary` tests, the SQLite schema test, `CollectionReport`, command return type, event emitter, and frontend decoder. Confirm that only normalized counters, opaque identities, timestamps, provider/state labels, and bounded categories cross the seam. Confirm `src-tauri/src/sources/file_watcher.rs`, `src-tauri/src/app/tray.rs`, `src-tauri/src/app/startup.rs`, and `src-tauri/src/app/window.rs` remain untouched.

- [ ] **Step 6: Review the final diff and commit only the approved slice.**

Run:

```powershell
git diff --check
git status --short --branch
git diff --stat 889f7ba..HEAD
```

Expected: only the console fix, runtime state, command/event seam, React subscription, tests, and this plan are present; no `.claude/` settings or generated provider data are staged.

## Acceptance Criteria for This Slice

1. The release executable opens without a console window.
2. Tauri startup creates a Rust-only runtime using the current user's fixed native Claude and Codex roots and explicit bounds.
3. The first collection pass persists normalized events/checkpoints through the existing transaction boundary and returns a post-commit summary.
4. Restarting against the same local index preserves totals and accepts no duplicate events.
5. Missing Claude or Codex sources degrade independently.
6. `get_usage_summary` and `usage-summary-changed` expose only the approved `UsageSummary` fields.
7. React receives the initial summary and valid later events, ignores payloads with forbidden/unknown fields, and cleans up its listener.
8. Rust tests, frontend tests, TypeScript/Vite build, integrated Tauri build, privacy checks, and release smoke verification pass.

## Explicitly Deferred Follow-up Slices

- A separate watcher/reconciliation plan will add filesystem notifications, 30-second reconciliation, retry/backoff, and repeated summary emission.
- A separate shell plan will add tray Show/Hide, Settings, Quit, transparent/framed window behavior, single-instance enforcement, startup registration, drag/position persistence, and opacity.
- A separate settings/source plan will add explicit provider-root overrides, WSL UNC selection, settings persistence, clear-index confirmation, and database backup/rebuild recovery.

## Plan Self-Review

### Spec coverage

- Startup/data flow: Task 2 resolves fixed roots and Task 3 runs discovery, adapter reads, transaction, summary, and post-commit emission.
- Provider independence: Task 2's missing-Codex test and the existing collection-core failure test cover independent outcomes.
- Restart/deduplication/partial-write rules: existing collection-core tests remain in the full gate; Task 2 adds the real SQLite-backed restart seam.
- Presentation contract: Task 3 preserves `get_usage_summary`; Task 4 implements the typed event consumer and relative footer.
- Privacy: Task 2's serialized fallback test, Task 3's wire allow-list, existing schema/privacy tests, and Task 4's runtime decoder cover the Rust-to-React seam.
- Windows shell baseline: Task 1 verifies the release PE subsystem; later shell behavior is explicitly outside this slice.

### Deferred requirements with an explicit owner

- Filesystem watcher and 30-second reconciliation: separate watcher plan.
- Tray, settings, window positioning, startup registration, single instance, and uninstall: separate shell plan.
- WSL explicit root selection and source settings: separate settings/source plan.

### Type consistency

- Rust `AppState::summary()` returns `UsageSummary`; the command returns the same type; the event emits the same type; TypeScript decodes the same camelCase fields.
- Rust `USAGE_SUMMARY_CHANGED_EVENT` and TypeScript `USAGE_SUMMARY_CHANGED_EVENT` both use `usage-summary-changed`.
- `Provider::display_name()` is used only for the user-facing summary provider; `Provider::as_str()` remains the database and source-health identifier.
- `AppState::collect_once` returns `CollectionReport`, but only `CollectionReport.summary` is passed to the command/event layer.

### Placeholder and ambiguity check

- No task relies on an unspecified file, unbounded scan, raw payload, or unstated dependency.
- The startup pass is explicitly one-shot; live repeated collection belongs to the deferred watcher slice.
- The plan contains no change to `main` or `dev` branch policy beyond the isolated release attribute and normal task commits.
