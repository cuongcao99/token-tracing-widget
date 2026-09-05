# Concurrent Windows and WSL Source Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let each Provider collect from its Windows and optional WSL source concurrently, with a Settings dialog that configures either source without disabling the other.

**Architecture:** Keep one provider-level enabled switch and add two typed root slots: automatic/custom Windows plus optional explicit WSL. Resolve both slots through the existing Rust discovery/collection path, aggregate them into one Provider source, and run one native observer worker per existing root. Keep the existing Tauri commands, adding a platform argument to the picker and typed root fields to source settings.

**Tech Stack:** Rust 2021, Tauri 2, SQLite via `rusqlite`, React 19, TypeScript, Vite, Vitest, plain CSS, existing Win32 directory observer.

**Spec:** `docs/superpowers/specs/2026-09-04-multi-source-wsl-design.md`

## Global Constraints

- Keep version one Windows 11-only, local-only, and metadata-only.
- Keep filesystem, collection, source discovery, and SQLite access in Rust.
- Accept absolute local Windows roots and `\\wsl.localhost\\<distribution>\\...` only; never invoke `wsl.exe`, enumerate distributions, or accept arbitrary network shares.
- Preserve provider adapters, normalized observations, cumulative-to-delta conversion, checkpoint identity, source-health independence, typed Tauri boundaries, and path-free observer signals.
- Reuse the existing `settings` key/value table; do not add a schema table or dependency.
- Keep Settings source paths visible only in the Settings source-editing flow and local source configuration/effective-root storage.
- Use `dev`-derived feature branch work and do not push or modify `main`.
- Work test-first: every behavior change gets a focused failing test before production code.

---

## File map

- Modify `src-tauri/src/sources/source_config.rs`: platform enum, dual root slots, validation, legacy-compatible constructor helpers.
- Modify `src-tauri/src/sources/provider_roots.rs`: resolve Windows plus optional WSL roots and watcher paths.
- Modify `src-tauri/src/sources/session_files.rs`: discover all configured roots while retaining the single-result compatibility wrapper.
- Modify `src-tauri/src/sources/file_watcher.rs`: retain multiple workers for one Provider.
- Modify `src-tauri/src/collection/coordinator.rs` and `src-tauri/src/collection/source_collection.rs`: collect all discoveries for one Provider and aggregate health/labels.
- Modify `src-tauri/src/app/runtime.rs`: build dual-root discoveries, watcher roots, and platform-specific picker paths.
- Modify `src-tauri/src/database/settings.rs` and `src-tauri/src/database/store.rs`: persist new keys and migrate the legacy root key.
- Modify `src-tauri/src/commands/source_settings.rs`: expose dual root fields and platform-aware picker input.
- Modify `src/lib/contracts/source-settings.ts`, `src/lib/source-settings.ts`, `src/lib/desktop/commands.ts`, and `src/lib/settings-model.ts`: update typed source settings and picker bridge.
- Modify `src/hooks/useSettingsController.ts`: pass platform to the picker and remove configured roots through the existing serialized persistence queue.
- Modify `src/components/settings/SourceSettingsSection.tsx`: render `Change source` and own dialog visibility.
- Create `src/components/settings/SourcePickerDialog.tsx`: accessible Windows/WSL source options.
- Modify `src/styles/settings/forms.module.css`: style the dialog and source options using existing semantic tokens.
- Modify focused frontend tests under `src/tests/components/settings`, `src/tests/hooks`, and `src/tests/lib`.
- Modify Rust tests under `src-tauri/src/sources/file_watcher.rs`, `src-tauri/tests/source_discovery.rs`, `src-tauri/tests/database.rs`, and `src-tauri/tests/collection_core.rs`.
- Modify `PRODUCT.md` and `CONTEXT.md` only after implementation is verified so current behavior documents the concurrent-root contract.

## Task 1: Define the dual-root Rust model and migration contract

**Files:**
- Modify: `src-tauri/src/sources/source_config.rs`
- Modify: `src-tauri/src/database/settings.rs`
- Modify: `src-tauri/src/database/store.rs`
- Test: source-config unit tests and `src-tauri/tests/database.rs`

**Interfaces:**
- Add `SourcePlatform::{Windows, Wsl}`.
- Add `SourceConfig::try_new_with_roots(provider, enabled, windows_root_override, wsl_root_override)`.
- Add `SourceConfig::windows_root_override()`, `wsl_root_override()`, and `with_root_override(platform, path)`.
- Keep `SourceConfig::try_new(provider, enabled, root_override)` as a compatibility constructor that classifies a WSL-shaped path into the WSL slot.
- Add `parse_windows_root` and `parse_wsl_root` while retaining `parse_explicit_root` for legacy parsing.

- [ ] **Step 1: Write the failing model and migration tests.**

Assert that a config can hold both roots, that cross-platform paths are
rejected, that legacy local/WSL keys load into the right slots, and that a
successful dual-root save deletes only the Provider's legacy root key.

```rust
let config = SourceConfig::try_new_with_roots(
    Provider::Claude,
    true,
    Some(PathBuf::from(r"C:\Users\tester\.claude\projects")),
    Some(PathBuf::from(r"\\wsl.localhost\Ubuntu\home\tester\.claude\projects")),
).unwrap();

assert!(config.windows_root_override().is_some());
assert!(config.wsl_root_override().is_some());
```

- [ ] **Step 2: Run focused Rust tests and verify the expected red failure.**

```text
cargo test --manifest-path src-tauri/Cargo.toml source_config
cargo test --manifest-path src-tauri/Cargo.toml --test database source_preferences
```

Expected: the new constructor/accessors and migration behavior are missing.

- [ ] **Step 3: Implement the smallest dual-root model and settings-key migration.**

Use the existing path validator. Store Windows and WSL overrides under their
new provider-scoped keys, classify a legacy `root_override` by WSL UNC shape,
and delete only that legacy key when saving the Provider. Reject WSL paths in
the Windows slot and local paths in the WSL slot without echoing values.

- [ ] **Step 4: Run focused tests and verify green.**

Run the two commands from Step 2 and confirm all existing source-settings tests
also pass.

- [ ] **Step 5: Commit the model slice.**

```text
git add src-tauri/src/sources/source_config.rs src-tauri/src/database/settings.rs src-tauri/src/database/store.rs src-tauri/tests/database.rs
git commit -m "feat: persist concurrent Windows and WSL roots"
```

## Task 2: Resolve and discover both roots through the Rust collection path

**Files:**
- Modify: `src-tauri/src/sources/provider_roots.rs`
- Modify: `src-tauri/src/sources/session_files.rs`
- Modify: `src-tauri/src/app/runtime.rs`
- Test: `src-tauri/tests/source_discovery.rs`
- Test: `src-tauri/tests/runtime_integration.rs`

**Interfaces:**
- Add `resolve_configured_roots(profile_root, config)` returning the Windows result and optional WSL result.
- Add `discover_configured_sources(profile_root, config, limits) -> Vec<DiscoveryResult>`.
- Add `configured_root_path_for(profile_root, config, platform)` for the picker.
- Add `watch_root_paths(profile_root, config) -> Vec<PathBuf>`.
- Keep existing singular functions as compatibility wrappers for current tests and callers.

- [ ] **Step 1: Write failing dual-root discovery and watcher-path tests.**

Create two synthetic roots, configure both, and assert two discovery results,
two configured labels, both file sets, and both watcher paths. Add a missing WSL
root case that still returns a persisted `not_detected` result without scanning
the profile root.

- [ ] **Step 2: Run the focused discovery tests and verify red.**

```text
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery dual_root
cargo test --manifest-path src-tauri/Cargo.toml --test runtime_integration dual_root
```

- [ ] **Step 3: Implement dual-root resolution and discovery.**

Always resolve the fixed Windows root (automatic or custom) and add the WSL
root only when configured. Map each root failure to its own sanitized
`DiscoveryResult`; do not change provider readers or file metadata sanitization.

- [ ] **Step 4: Wire runtime collection inputs and watcher paths.**

Build one Provider input containing all its discoveries and return all existing
watch paths. Use the platform-specific configured path for the native picker.

- [ ] **Step 5: Run focused discovery and integration tests.**

```text
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery
cargo test --manifest-path src-tauri/Cargo.toml --test runtime_integration
```

- [ ] **Step 6: Commit the source-resolution slice.**

```text
git add src-tauri/src/sources/provider_roots.rs src-tauri/src/sources/session_files.rs src-tauri/src/app/runtime.rs src-tauri/tests/source_discovery.rs src-tauri/tests/runtime_integration.rs
git commit -m "feat: discover concurrent provider roots"
```

## Task 3: Aggregate collection and support multiple native observers

**Files:**
- Modify: `src-tauri/src/collection/coordinator.rs`
- Modify: `src-tauri/src/collection/source_collection.rs`
- Modify: `src-tauri/src/sources/file_watcher.rs`
- Test: `src-tauri/tests/collection_core.rs`
- Test: `src-tauri/src/sources/file_watcher.rs` tests

**Interfaces:**
- Extend `ProviderSource` with multiple discoveries while retaining the single-discovery constructors.
- Keep one `SourceUpdate` and one `SourceHealth` entry per Provider.
- Make `SourceObserver` retain a vector of workers per Provider and stop all of them through `stop_provider`.

- [ ] **Step 1: Write failing tests for collection merge and observer fan-out.**

Assert that two Claude discoveries produce both event sets, one combined source
update, one usable Provider health entry, and a combined total. On Windows,
start two roots for Claude, write to both, and assert both changes arrive as
provider-only signals.

- [ ] **Step 2: Run focused tests and verify red.**

```text
cargo test --manifest-path src-tauri/Cargo.toml --test collection_core dual_root
cargo test --manifest-path src-tauri/Cargo.toml --lib sources::file_watcher observer
```

- [ ] **Step 3: Implement collection aggregation.**

Iterate every discovery with the existing checkpoint, adapter, byte-budget,
delta, and diagnostic logic. Merge discovery/reader health so one usable root
keeps the Provider usable; preserve one source update with a deterministic
joined configured-root label.

- [ ] **Step 4: Implement observer fan-out and refresh lifecycle.**

Change the worker map to `BTreeMap<Provider, Vec<ProviderWorker>>`. `start_provider`
adds one worker; `stop_provider` joins every worker for that Provider. Update
the live-controller refresh path to stop the old Provider workers before adding
the replacement set.

- [ ] **Step 5: Run focused collection and observer tests.**

```text
cargo test --manifest-path src-tauri/Cargo.toml --test collection_core
cargo test --manifest-path src-tauri/Cargo.toml --lib sources::file_watcher
```

- [ ] **Step 6: Commit the collection/observer slice.**

```text
git add src-tauri/src/collection/coordinator.rs src-tauri/src/collection/source_collection.rs src-tauri/src/sources/file_watcher.rs src-tauri/tests/collection_core.rs
git commit -m "feat: collect and watch multiple roots per provider"
```

## Task 4: Update typed Tauri source settings and picker commands

**Files:**
- Modify: `src-tauri/src/commands/source_settings.rs`
- Modify: `src/lib/contracts/source-settings.ts`
- Modify: `src/lib/source-settings.ts`
- Modify: `src/lib/desktop/commands.ts`
- Modify: `src/lib/settings-model.ts`
- Test: command tests, `src/tests/lib/source-settings.test.ts`, and `src/tests/lib/desktop-boundary.test.ts`

**Interfaces:**
- `SourceSettingsInput/View` fields: `provider`, `enabled`, `windows_root`, `wsl_root` (serialized as `windowsRoot`, `wslRoot`).
- `pick_source_root(provider, platform)` and `pickSourceRoot(provider, platform)`.
- Frontend `SourcePlatform = "windows" | "wsl"`.

- [ ] **Step 1: Write failing contract and command tests.**

Assert exact dual-root snapshots, rejection of old raw fields, and the picker
payload `{ provider: "claude", platform: "wsl" }`.

- [ ] **Step 2: Run focused frontend/Rust tests and verify red.**

```text
npm test -- --run src/tests/lib/source-settings.test.ts src/tests/lib/desktop-boundary.test.ts
cargo test --manifest-path src-tauri/Cargo.toml source_settings
```

- [ ] **Step 3: Implement the typed fields and platform-aware picker.**

Use `SourcePlatform` to select the picker initial path and update only that
root slot. Keep the existing sanitized errors and folder-picker cancellation
semantics.

- [ ] **Step 4: Run focused tests and verify green.**

Run the commands from Step 2 and confirm the full source-settings bridge suite
passes.

- [ ] **Step 5: Commit the typed boundary slice.**

```text
git add src-tauri/src/commands/source_settings.rs src/lib/contracts/source-settings.ts src/lib/source-settings.ts src/lib/desktop/commands.ts src/lib/settings-model.ts src/tests/lib/source-settings.test.ts src/tests/lib/desktop-boundary.test.ts
git commit -m "feat: expose platform-specific source settings"
```

## Task 5: Add the Change source dialog and controller actions

**Files:**
- Modify: `src/hooks/useSettingsController.ts`
- Modify: `src/components/settings/SourceSettingsSection.tsx`
- Create: `src/components/settings/SourcePickerDialog.tsx`
- Modify: `src/styles/settings/forms.module.css`
- Test: `src/tests/components/settings/SettingsScreen.behavior.test.tsx`
- Test: `src/tests/components/settings/SettingsScreen.structure.test.tsx`
- Test: `src/tests/hooks/useSettingsController.edge.test.ts`

**Interfaces:**
- `SourceSettingsSection` receives `onChooseRoot(provider, platform)` and `onClearRoot(provider, platform)`.
- The dialog exposes an accessible `role="dialog"`, `aria-modal="true"`, a close button, Windows and WSL options, and platform-specific picker actions.

- [ ] **Step 1: Write failing Settings tests.**

Cover `Change source`, both option labels, the explicit “both can be active” copy,
platform-aware picker calls, WSL removal, Escape/close behavior, and the
unchanged provider-level collection switch.

- [ ] **Step 2: Run the focused Settings tests and verify red.**

```text
npm test -- --run src/tests/components/settings/SettingsScreen.behavior.test.tsx src/tests/components/settings/SettingsScreen.structure.test.tsx src/tests/hooks/useSettingsController.edge.test.ts
```

- [ ] **Step 3: Implement the dialog and controller plumbing.**

Keep dialog visibility local to `SourceSettingsSection`. Use the existing
controller persistence queue for root removal; the native picker continues to
persist selected paths through its existing command. Show automatic Windows,
custom Windows, configured WSL, and not-configured WSL states without exposing
paths outside Settings.

- [ ] **Step 4: Add semantic-token CSS and verify focused tests.**

Use the existing Claude canvas/card/accent/line/elevation/focus tokens. Do not
add a CSS framework, modal dependency, or new global selector.

- [ ] **Step 5: Commit the Settings slice.**

```text
git add src/hooks/useSettingsController.ts src/components/settings/SourceSettingsSection.tsx src/components/settings/SourcePickerDialog.tsx src/styles/settings/forms.module.css src/tests/components/settings src/tests/hooks/useSettingsController.edge.test.ts
git commit -m "feat: configure Windows and WSL sources in Settings"
```

## Task 6: Update product documentation and run the full verification gate

**Files:**
- Modify: `PRODUCT.md`
- Modify: `CONTEXT.md`
- Modify: relevant existing source/Settings tests only if integration exposes a real regression.

- [ ] **Step 1: Update current-state documentation.**

Document that Windows is always the default source, WSL is optional and
explicit, both can be collected concurrently, and auto-discovery is excluded.

- [ ] **Step 2: Run frontend verification.**

```text
npm test -- --run
npm run build
```

- [ ] **Step 3: Run Rust verification.**

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 4: Run the integrated Windows debug package.**

```text
npm run tauri build -- --debug
```

Confirm the executable and NSIS installer are produced. In the packaged app,
verify the Settings dialog, native Windows/WSL folder-picker navigation,
simultaneous root configuration, cancellation, removal, watcher refresh, and
absence of source paths in overlay/summary payloads.

- [ ] **Step 5: Review the diff and branch state.**

```text
git diff --check
git status --short --branch
git diff --stat dev...HEAD
```

Stage only intended source, tests, and documentation; leave generated output
and local review artifacts ignored.
