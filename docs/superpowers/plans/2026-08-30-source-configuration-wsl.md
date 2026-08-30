# Source Configuration and WSL Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add persisted per-provider source configuration, safe local/WSL root resolution, and live watcher reload while preserving the metadata-only Windows runtime.

**Architecture:** Reuse SQLite's existing `settings` key/value table for preferences and keep `sources` as the collection-owned health/effective-root mirror. Add a Rust-only `SourceConfig` model, route native and explicit roots through one resolver, carry configured-root labels separately from discovery results, and refresh watcher workers with a path-free signal after successful persistence.

**Tech Stack:** Rust 2021, Tauri 2, SQLite via `rusqlite`, Windows filesystem APIs, existing React/TypeScript/Vite frontend and Vitest gates.

**Spec:** `docs/superpowers/specs/2026-08-30-source-configuration-wsl-design.md`

## Global Constraints

- Keep version one local-only and Windows 11-only.
- Keep filesystem, collection, and SQLite access in Rust.
- Preserve metadata-only collection: prompts, responses, reasoning, tool payloads, credentials, repository contents, and working directories never enter normalized events, SQLite, diagnostics, or frontend payloads.
- Keep provider-specific formats behind adapters and enforce normalization, delta conversion, deduplication, and checkpoint invariants in the collection core.
- Store source preferences in the existing `settings` table; add no schema table or migration.
- Accept only absolute local Windows roots or `\\wsl.localhost\\<distribution>\\...`; never invoke `wsl.exe`, enumerate distributions, or accept arbitrary network shares.
- Add no network client, telemetry, sidecar, background service, frontend state library, CSS framework, or ORM.
- Add no React settings window, visual redesign, always-below behavior, startup registration, clear-index flow, or database rebuild flow.
- Work test-first: every behavior change gets a focused failing Rust test, a red run, a minimal implementation, and a green run.
- Keep the overlay contract typed and path-free; explicit roots may remain only in local settings/source storage and the future settings flow.

---

### Task 1: Add the source configuration model and root syntax validator

**Files:**
- Create: `src-tauri/src/sources/source_config.rs`
- Modify: `src-tauri/src/sources/mod.rs`

**Interfaces:**
- `SourceConfig::try_new(provider: Provider, enabled: bool, root_override: Option<PathBuf>) -> Result<SourceConfig, SourceConfigError>` validates an optional override without filesystem I/O.
- `SourceConfig::defaults(provider: Provider) -> SourceConfig` returns enabled + automatic native selection.
- `SourceConfig::provider`, `enabled`, `root_override`, and `configured_root_label` expose typed internal values.
- `SourceConfigSet::defaults`, `get`, `replace`, `is_enabled`, and `enabled_providers` provide deterministic Claude/Codex access.
- `parse_explicit_root(raw: &str) -> Result<PathBuf, SourceConfigError>` accepts drive-qualified absolute paths and `\\wsl.localhost\\<distribution>\\...` only.
- `LoadedSourceConfigs { configs: SourceConfigSet, invalid_providers: Vec<Provider> }` will be defined here for Task 2.

- [ ] **Step 1: Write the failing tests**

Add tests in `source_config.rs`:

```rust
#[test]
fn defaults_enable_both_providers_and_use_native_labels() {
    let configs = SourceConfigSet::defaults();
    assert!(configs.is_enabled(Provider::Claude));
    assert!(configs.is_enabled(Provider::Codex));
    assert_eq!(configs.get(Provider::Claude).configured_root_label(), ".claude/projects");
    assert_eq!(configs.get(Provider::Codex).configured_root_label(), ".codex/sessions");
}

#[cfg(windows)]
#[test]
fn explicit_root_accepts_approved_wsl_unc_shape() {
    let path = parse_explicit_root(r"\\wsl.localhost\Ubuntu\home\user\.claude\projects").unwrap();
    assert_eq!(path.to_string_lossy(), r"\\wsl.localhost\Ubuntu\home\user\.claude\projects");
}

#[cfg(windows)]
#[test]
fn explicit_root_rejects_arbitrary_unc_relative_and_device_paths() {
    assert!(parse_explicit_root(r"\\server\share\sessions").is_err());
    assert!(parse_explicit_root(r".claude\projects").is_err());
    assert!(parse_explicit_root(r"\\?\C:\sessions").is_err());
    assert!(parse_explicit_root(r"C:\sessions\..\outside").is_err());
}
```

- [ ] **Step 2: Run the focused tests and verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml source_config::tests -- --nocapture`

Expected: FAIL because the module and requested model/parser do not exist.

- [ ] **Step 3: Implement the minimal model and parser**

Export the module. Keep fields private, use stable `[Claude, Codex]` ordering, and expose constructors/accessors only. Reject empty/NUL strings, relative paths, URI-looking strings, device prefixes, arbitrary UNC servers, empty distribution components, `.` components, and `..` traversal. Use fixed sanitized error categories: `EmptyRoot`, `NulByte`, `RelativeRoot`, `UnsupportedUnc`, `DevicePath`, `ParentTraversal`, and `InvalidRoot`.

- [ ] **Step 4: Run the focused tests and verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml source_config::tests -- --nocapture`

Expected: PASS, including Windows-only path cases on this host.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/sources/source_config.rs src-tauri/src/sources/mod.rs
git commit -m "feat: add source configuration model"
```

### Task 2: Persist source preferences through SQLite

**Files:**
- Modify: `src-tauri/src/database/settings.rs`
- Modify: `src-tauri/src/database/connection.rs`
- Modify: `src-tauri/tests/database.rs`

**Interfaces:**
- `IndexStore::load_source_configs(&self) -> Result<LoadedSourceConfigs, StorageError>` reads only the four source keys and defaults malformed values independently.
- `IndexStore::save_source_config(&mut self, config: &SourceConfig) -> Result<(), StorageError>` writes one provider's enabled key and upserts/deletes its override key transactionally.
- Keys are `source.<provider>.enabled` and `source.<provider>.root_override`; unknown keys remain untouched.

- [ ] **Step 1: Write failing tests**

Add round-trip and malformed-value tests:

```rust
#[test]
fn source_preferences_round_trip_and_remove_override() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("index.sqlite");
    let mut database = IndexStore::open(&path).unwrap();
    let config = SourceConfig::try_new(
        Provider::Claude,
        false,
        Some(PathBuf::from(r"C:\Users\tester\.claude\projects")),
    ).unwrap();
    database.save_source_config(&config).unwrap();
    assert_eq!(database.load_source_configs().unwrap().configs.get(Provider::Claude), &config);

    let automatic = SourceConfig::try_new(Provider::Claude, true, None).unwrap();
    database.save_source_config(&automatic).unwrap();
    assert_eq!(database.load_source_configs().unwrap().configs.get(Provider::Claude), &automatic);
}

#[test]
fn malformed_source_setting_defaults_only_its_provider() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("index.sqlite");
    let database = IndexStore::open(&path).unwrap();
    drop(database);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute(
        "INSERT INTO settings(setting_key, setting_value) VALUES (?1, ?2)",
        ["source.claude.enabled", "not-a-bool"],
    ).unwrap();
    drop(connection);

    let loaded = IndexStore::open(&path).unwrap().load_source_configs().unwrap();
    assert!(loaded.configs.is_enabled(Provider::Claude));
    assert!(loaded.configs.is_enabled(Provider::Codex));
    assert_eq!(loaded.invalid_providers, vec![Provider::Claude]);
}
```

- [ ] **Step 2: Run the tests and verify the missing-accessor failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test database source_preferences_round_trip_and_remove_override -- --nocapture`

Expected: FAIL because the two `IndexStore` methods do not exist.

- [ ] **Step 3: Implement parameterized settings access**

Query only known keys. Parse enabled values `0`/`1`; parse override values through `parse_explicit_root`; default malformed fields and record the provider once in `invalid_providers`. Never log, store, or return malformed input. Save enabled with upsert, save an override with upsert, and remove only that provider's override when switching to automatic. Map SQLite read/write errors to existing `StorageError` variants.

- [ ] **Step 4: Run all database tests and verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test database -- --nocapture`

Expected: PASS for existing atomicity/privacy tests and the new settings tests.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/database/settings.rs src-tauri/src/database/connection.rs src-tauri/tests/database.rs
git commit -m "feat: persist source preferences in sqlite"
```

### Task 3: Resolve configured roots and discover explicit sources

**Files:**
- Modify: `src-tauri/src/sources/provider_roots.rs`
- Modify: `src-tauri/src/sources/session_files.rs`
- Modify: `src-tauri/tests/source_discovery.rs`

**Interfaces:**
- `ProviderRoot { provider, configured_root: String, filesystem_path: PathBuf }` exposes `provider`, `configured_root`, and Rust-only `filesystem_path` accessors.
- `resolve_configured_root(profile_root: &Path, config: &SourceConfig) -> Result<ProviderRoot, RootError>` resolves automatic or explicit roots.
- `resolve_native_root` remains a wrapper around `SourceConfig::defaults`.
- `DiscoveryStatus` gains `Disabled`; `DiscoveryResult::configured_root(&self) -> &str` replaces `root_relative`.
- `discover_configured_source(profile_root, config, limits) -> DiscoveryResult` is the single configured entry point; `discover_native_sources` remains a default wrapper.

- [ ] **Step 1: Write failing discovery tests**

Add tests for an explicit temporary root, a missing root, and a disabled root:

```rust
#[test]
fn explicit_existing_root_is_discovered_with_its_configured_label() {
    let profile = tempfile::tempdir().unwrap();
    let root = profile.path().join("custom-source");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("session.jsonl"), b"metadata only").unwrap();
    let config = SourceConfig::try_new(Provider::Claude, true, Some(root.clone())).unwrap();
    let result = discover_configured_source(profile.path(), &config, DiscoveryLimits::new(10, 1_000));
    assert_eq!(result.status(), DiscoveryStatus::Detected);
    assert_eq!(result.configured_root(), root.to_string_lossy());
    assert_eq!(result.files().len(), 1);
}

#[test]
fn missing_explicit_root_reports_not_detected_without_scanning_profile() {
    let profile = tempfile::tempdir().unwrap();
    let root = profile.path().join("not-mounted-yet");
    let config = SourceConfig::try_new(Provider::Claude, true, Some(root.clone())).unwrap();
    let result = discover_configured_source(profile.path(), &config, DiscoveryLimits::new(10, 1_000));
    assert_eq!(result.status(), DiscoveryStatus::NotDetected);
    assert!(result.files().is_empty());
    assert_eq!(result.configured_root(), root.to_string_lossy());
}

#[test]
fn disabled_source_never_walks_its_root() {
    let profile = tempfile::tempdir().unwrap();
    let root = profile.path().join("disabled-source");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("session.jsonl"), b"metadata only").unwrap();
    let config = SourceConfig::try_new(Provider::Claude, false, Some(root)).unwrap();
    let result = discover_configured_source(profile.path(), &config, DiscoveryLimits::new(10, 1_000));
    assert_eq!(result.status(), DiscoveryStatus::Disabled);
    assert!(result.files().is_empty());
}
```

- [ ] **Step 2: Run the tests and verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery explicit_existing_root_is_discovered_with_its_configured_label -- --nocapture`

Expected: FAIL because configured resolution and the new result label do not exist.

- [ ] **Step 3: Implement root resolution**

Give `ProviderRoot` an owned label. Keep native roots profile-bound with existing reparse checks. For explicit roots, use the syntax-validated `PathBuf`; map missing to `NotDetected`, require existing paths to be directories, and validate the selected root against reparse points. Keep child validation in the existing walker.

- [ ] **Step 4: Implement configured discovery**

Change `DiscoveryResult` to own `configured_root: String`. Add the disabled result before any filesystem call. Resolve enabled roots independently, map root failures to existing statuses, and pass the resolved root to the existing bounded walker. Preserve the native convenience wrapper and update all native tests to the renamed accessor.

- [ ] **Step 5: Run source-discovery tests and verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery -- --nocapture`

Expected: PASS for native, local explicit, missing, disabled, limits, sanitization, WSL-shaped syntax, and reparse tests.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/sources/provider_roots.rs src-tauri/src/sources/session_files.rs src-tauri/tests/source_discovery.rs
git commit -m "feat: discover configured provider roots"
```

### Task 4: Carry configuration through collection and aggregation

**Files:**
- Modify: `src-tauri/src/collection/mod.rs`
- Modify: `src-tauri/tests/collection_core.rs`

**Interfaces:**
- `ProviderSource::new(enabled: bool, discovery: DiscoveryResult, adapter: &dyn ProviderAdapter) -> ProviderSource` remains the compatibility constructor and derives the discovery label with no pending settings marker.
- `ProviderSource::with_configured_root(enabled: bool, configured_root: String, settings_issue: bool, discovery: DiscoveryResult, adapter: &dyn ProviderAdapter) -> ProviderSource` owns the explicit source-update label and one pending settings diagnostic marker.
- `compute_summary(rows, source_health, enabled_providers: &[Provider], clock) -> UsageSummary` filters event rows before active and daily totals.
- Disabled collection returns `SourceHealth` state `disabled` and performs no adapter read/checkpoint work.

- [ ] **Step 1: Write the failing aggregation test**

Add a summary case with Claude enabled and Codex disabled:

```rust
#[test]
fn disabled_provider_events_do_not_enter_summary_totals() {
    let rows = SummaryRows { events: vec![
        UsageEvent::for_test(Provider::Claude, "claude-session", "2026-01-01T10:00:00Z", 20),
        UsageEvent::for_test(Provider::Codex, "codex-session", "2026-01-01T10:00:01Z", 30),
    ]};
    let health = vec![
        SourceHealth::detected(Provider::Claude),
        SourceHealth::new(Provider::Codex, "disabled"),
    ];
    let summary = compute_summary(
        &rows, &health, &[Provider::Claude],
        &FixedClock::new("2026-01-01T10:00:30Z", "2026-01-01"),
    );
    assert_eq!(summary.today_tokens, 20);
    assert_eq!(summary.provider.as_deref(), Some("Claude Code"));
}
```

Add a coordinator regression that uses an explicit configured-root label and asserts the resulting `SourceUpdate` does not revert to `.claude/projects`. Add `source_updates: Vec<SourceUpdate>` to the existing in-memory test store and append each batch update before returning from `apply_batch`.

- [ ] **Step 2: Run collection tests and verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test collection_core disabled_provider -- --nocapture`

Expected: FAIL because `compute_summary` has no enabled-provider argument and source updates still read the native-only discovery field.

- [ ] **Step 3: Implement source labels, disabled state, and settings diagnostics**

Add the owned label and `settings_issue` marker to `ProviderSource`, keep `new` as a compatibility constructor, and use `with_configured_root` for runtime/configured sources. Write `SourceUpdate.configured_root` from the owned label, return `disabled` before discovery reads, and map `DiscoveryStatus::Disabled` to `disabled`. When the marker is set, add one `DiagnosticUpdate { category: "invalid_settings" }` without including the setting key/value/path. Runtime supplies the marker only for providers whose malformed persisted setting has not yet been acknowledged by a successful collection.

- [ ] **Step 4: Implement enabled-provider filtering**

Derive enabled providers from the ordered source list. Filter `SummaryRows.events` by that list before `compute_active_provider` and `compute_today_total`; retain all health entries and all stored historical events.

- [ ] **Step 5: Run all collection tests and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test collection_core -- --nocapture`

Expected: PASS for all existing delta/dedupe/partial-write/rotation/failure tests and the new disabled-source tests.

```powershell
git add src-tauri/src/collection/mod.rs src-tauri/tests/collection_core.rs
git commit -m "feat: honor enabled sources in collection totals"
```

### Task 5: Load and update configuration in `AppState`

**Files:**
- Modify: `src-tauri/src/app/runtime.rs`
- Modify: `src-tauri/tests/runtime_integration.rs`

**Interfaces:**
- `Runtime` stores `source_configs: SourceConfigSet` and pending invalid-settings providers.
- `AppState::source_config(&self, provider: Provider) -> Result<SourceConfig, RuntimeError>` returns a clone.
- `AppState::update_source_config(&self, config: SourceConfig) -> Result<(), RuntimeError>` persists first, then updates shared memory.
- `Runtime::watch_roots` resolves only enabled configured roots and includes only existing validated directories.
- Add sanitized `SettingsRead` initialization and `Settings(StorageError)` runtime errors.

- [ ] **Step 1: Write failing runtime tests**

Seed the database with `IndexStore::save_source_config` before constructing `AppState`; assert a disabled Codex source contributes no tokens and reports `disabled`. Add a second test that calls `update_source_config`, reopens the database, and compares the persisted and in-memory configs.

- [ ] **Step 2: Run runtime tests and verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_integration persisted -- --nocapture`

Expected: FAIL because initialization always builds default sources and `AppState` lacks the config API.

- [ ] **Step 3: Load settings during initialization**

After opening SQLite, load `LoadedSourceConfigs`, map read failure to `RuntimeInitError::SettingsRead`, store configs and invalid-provider markers, then move the store into the coordinator. The markers are pending one-time sanitized `invalid_settings` diagnostics; malformed values themselves are never retained.

- [ ] **Step 4: Build discovery/watch roots from config**

Use `discover_configured_source` for both providers. Construct `ProviderSource::with_configured_root` with each config's enabled state and label. Iterate only enabled configs in `watch_roots`; missing roots remain eligible for reconciliation after they appear.

- [ ] **Step 5: Implement write-before-memory update**

Add `CollectionCoordinator<IndexStore>::save_source_config` as a narrow forwarding method. `Runtime::update_source_config` calls it first, then replaces the config and clears that provider's invalid marker. A failed write leaves config and roots unchanged.

- [ ] **Step 6: Clear pending settings diagnostics only after commit**

Build each `ProviderSource` with `settings_issue = invalid_providers.contains(&provider)`. After `CollectionCoordinator::collect` returns `Ok`, remove the affected providers from the pending list; if collection or storage fails, keep the markers so the next successful retry records them.

- [ ] **Step 7: Run runtime tests and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_integration -- --nocapture`

Expected: PASS for native collection, restart deduplication, independent roots, persisted disable/override behavior, and update persistence.

```powershell
git add src-tauri/src/app/runtime.rs src-tauri/tests/runtime_integration.rs src-tauri/src/collection/mod.rs
git commit -m "feat: load configured sources in runtime"
```

### Task 6: Refresh watcher roots after configuration changes

**Files:**
- Modify: `src-tauri/src/sources/file_watcher.rs`
- Modify: `src-tauri/src/app/live_collection.rs`

**Interfaces:**
- Add path-free `WatchSignal::ConfigurationChanged`.
- `LiveCollectionHandle::request_source_refresh(&self) -> bool` sends that signal unless shut down.
- `LiveCollectionLoop::on_signal` debounces `ConfigurationChanged`.
- `LiveCollectionLoop::run` calls `watcher.replace_roots(self.backend.watch_roots())` for that signal before the debounced collection.

- [ ] **Step 1: Write failing live-loop and handle tests**

Assert that `on_signal(WatchSignal::ConfigurationChanged, now)` returns true and sets the notification deadline. Build a test handle around an `mpsc` channel, call `request_source_refresh`, and assert the received enum is exactly `ConfigurationChanged` with no path field.

- [ ] **Step 2: Run focused tests and verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml live_collection::tests::configuration_changed_marks_collection_due_without_carrying_a_path -- --nocapture`

Expected: FAIL because the signal variant and handle method do not exist.

- [ ] **Step 3: Implement the signal and refresh path**

Add the enum variant and sender method. In the live loop, mark the scheduler changed for the new signal; in `run`, replace roots from the shared backend before normal debounce processing. Preserve existing notification, reconciliation, retry, and shutdown semantics.

- [ ] **Step 4: Add the owning orchestration helper**

Add an internal helper beside `LiveCollectionHandle` that calls `AppState::update_source_config` and calls `request_source_refresh` only after `Ok(())`. Do not register a Tauri command or send roots to React in this slice.

- [ ] **Step 5: Run live tests and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml live_collection -- --nocapture`

Expected: PASS for refresh, debounce, reconciliation, retry, publisher failure, and shutdown tests.

```powershell
git add src-tauri/src/sources/file_watcher.rs src-tauri/src/app/live_collection.rs
git commit -m "feat: refresh watcher roots after source changes"
```

### Task 7: Verify privacy boundaries and all repository gates

**Files:**
- Modify: `src-tauri/tests/database.rs` and `src-tauri/tests/runtime_integration.rs` only for regression assertions

- [ ] **Step 1: Add privacy regression assertions**

Store a synthetic private explicit root, collect, serialize `UsageSummary`, and assert the path and forbidden raw-field names are absent. Seed malformed settings and assert diagnostics contain only `invalid_settings`, never the bad value/path.

- [ ] **Step 2: Run the focused privacy tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test database privacy -- --nocapture`
Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_integration privacy -- --nocapture`

Expected: PASS with no path or raw payload crossing the summary/diagnostic boundary.

- [ ] **Step 3: Run all required gates**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm test -- --run
npm run build
npm run tauri build -- --debug
```

Expected: every command succeeds; frontend build remains unchanged and integrated Tauri build links the new Rust core.

- [ ] **Step 4: Review scope and branch cleanliness**

```powershell
git diff --check origin/dev...HEAD
git diff --stat origin/dev...HEAD
git status --short --branch
```

Confirm no `.claude/` settings, raw fixture content, frontend path payload, network code, or unrelated UI edits are present. Commit only any gate-specific correction with a scoped message; otherwise leave the branch clean and report the commits, gates, and explicit deferrals.
