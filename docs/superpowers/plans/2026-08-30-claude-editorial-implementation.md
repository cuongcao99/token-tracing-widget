# Claude Editorial Overlay and Frontend Organization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the reviewed Claude-editorial variant D as the real multi-provider overlay and Settings surface while reorganizing the frontend into focused components, hooks, styles, bridge modules, and tests.

**Architecture:** Keep Rust as the owner of normalized event aggregation, SQLite persistence, source configuration, and the two typed Tauri boundaries. Add a small provider-summary derivation beside the existing usage aggregation and persist widget-only preferences in the existing key/value settings table. The React webviews consume typed summaries/preferences through focused hooks, render separate widget/settings components, and share plain CSS tokens split by surface.

**Tech Stack:** Tauri 2, Rust, SQLite through the existing `IndexStore`, React 19, TypeScript, Vite, Vitest, Testing Library, plain CSS, and the existing `@tauri-apps/api` bridge.

**Spec:** `docs/superpowers/specs/2026-08-30-claude-editorial-multi-provider-design-update.md`, preserving the privacy and lifecycle requirements in `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md` and source settings behavior in `docs/superpowers/specs/2026-08-30-source-configuration-wsl-design.md`.

## Global Constraints

- Keep version one local-only and Windows 11-only.
- Keep filesystem, collection, and SQLite access in Rust. React receives typed summaries, widget preferences, and configured source roots only in Settings flows.
- Preserve metadata-only collection: prompts, responses, reasoning, tool payloads, credentials, repository contents, and working directories never enter normalized events, SQLite, diagnostics, or frontend payloads.
- Keep provider-specific formats behind adapters and enforce normalization, delta conversion, deduplication, and checkpoint invariants in the collection core.
- Add no network client, telemetry, sidecar, background service, frontend state library, CSS framework, or ORM.
- Work on `codex/overlay-apple-redesign`/`dev` lineage; leave `main` untouched.
- Keep source settings commands and root validation compatible; blank root means the automatic native path and invalid-root errors remain sanitized.
- Use TDD for behavior changes and run the same proof after structural refactors.
- Do not modify `.claude/` settings or user-provided design files.

---

### Task 1: Lock the Rust summary and widget-preference contracts with failing tests

**Files:**
- Create: `src-tauri/src/types/widget_settings.rs`
- Create: `src-tauri/src/types/provider_usage_summary.rs`
- Create: `src-tauri/src/commands/widget_settings.rs`
- Modify: `src-tauri/src/types/mod.rs`
- Modify: `src-tauri/src/types/usage_summary.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/types/provider_usage_summary.rs`
- Test: `src-tauri/src/commands/widget_settings.rs`

**Interfaces:**
- Produces `ProviderUsageSummary`, `WidgetSettings`, `VisibleProviderSetting`, `WidgetSettingsSnapshot`, `get_widget_settings`, `update_widget_settings`, and `WIDGET_SETTINGS_CHANGED_EVENT`.
- `UsageSummary.providers` is a fixed-order Claude/Codex collection with only normalized counters, Provider IDs, states, and timestamps.

- [x] **Step 1: Write the failing Rust serialization tests.**

Add tests that describe the new wire shapes before defining the types:

```rust
#[test]
fn provider_summary_serializes_only_normalized_fields() {
    let summary = ProviderUsageSummary::new(
        Provider::Claude,
        UsageState::Idle,
        Some(20),
        40,
        Some("2026-01-01T00:00:00Z".to_owned()),
    );
    let object = serde_json::to_value(summary).unwrap();
    assert_eq!(
        object.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["currentSessionTokens", "lastUpdatedAt", "provider", "state", "todayTokens"]
    );
    assert!(!object.to_string().contains("rawRecord"));
}

#[test]
fn widget_settings_serialize_fixed_provider_visibility_and_dark_mode() {
    let snapshot = WidgetSettingsSnapshot::defaults();
    let object = serde_json::to_value(snapshot).unwrap();
    assert_eq!(object["darkMode"], true);
    assert_eq!(object["visibleProviders"].as_array().unwrap().len(), 2);
}

#[test]
fn widget_input_rejects_unknown_raw_fields() {
    let value = serde_json::json!({
        "visibleProviders": [],
        "darkMode": true,
        "prompt": "private text"
    });
    assert!(serde_json::from_value::<WidgetSettingsInput>(value).is_err());
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml provider_usage_summary widget_settings`

Expected: FAIL because the new types and command input do not exist.

- [x] **Step 2: Verify the RED failure is about missing symbols.**

Run the focused command again and record that it fails before any implementation code is added. Fix only test syntax if the failure is unrelated to the missing contract.

- [x] **Step 3: Implement the minimal serializable types and defaults.**

Define `ProviderUsageSummary` with camelCase serialization and optional token/timestamp fields. Define `WidgetSettingsSnapshot` with exactly two unique Provider entries and a boolean `dark_mode`; make `defaults()` return both visible and dark mode enabled. Keep `WidgetSettingsInput` strict with `deny_unknown_fields`, and validate Provider IDs, duplicate entries, and the two-entry invariant in the command boundary.

- [x] **Step 4: Register the new module and event constant.**

Export `provider_usage_summary` and `widget_settings` from their module files, register `commands::widget_settings::{get_widget_settings, update_widget_settings}` in `tauri::generate_handler!`, and keep the existing source/usage command names unchanged.

- [x] **Step 5: Run the focused Rust tests.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml provider_usage_summary widget_settings`

Expected: PASS with no path, raw record, or prompt field in the serialized contract.

---

### Task 2: Derive independent Provider totals and states in the collection core

**Files:**
- Create: `src-tauri/src/usage/provider_summary.rs`
- Modify: `src-tauri/src/usage/mod.rs`
- Modify: `src-tauri/src/collection/mod.rs`
- Modify: `src-tauri/src/types/usage_summary.rs`
- Modify: `src-tauri/tests/collection_core.rs`
- Modify: `src-tauri/tests/runtime_integration.rs`
- Test: `src-tauri/src/usage/provider_summary.rs`

**Interfaces:**
- Consumes `SummaryRows`, `SourceHealth`, enabled Provider IDs, local day, and collection timestamp.
- Produces `compute_provider_summary(provider, events, health, now, local_day) -> ProviderUsageSummary` and a `UsageSummary` with `providers`, aggregate `today_tokens`, and the existing active-provider fields.

- [x] **Step 1: Write failing provider aggregation tests.**

Use real `UsageEvent::for_test` values, not mocks:

```rust
#[test]
fn computes_session_and_today_totals_per_provider() {
    let events = vec![
        UsageEvent::for_test(Provider::Claude, "claude-session", "2026-01-01T00:00:01Z", 20),
        UsageEvent::for_test(Provider::Claude, "claude-session", "2026-01-01T00:00:02Z", 22),
        UsageEvent::for_test(Provider::Codex, "codex-session", "2026-01-01T00:00:03Z", 10),
    ];
    let result = compute_provider_summary(
        Provider::Claude,
        &events,
        Some(&SourceHealth::detected(Provider::Claude)),
        "2026-01-01T00:00:04Z",
        "2026-01-01",
    );
    assert_eq!(result.current_session_tokens, Some(42));
    assert_eq!(result.today_tokens, 42);
    assert_eq!(result.state, UsageState::Active);
}

#[test]
fn preserves_idle_provider_totals_and_marks_missing_source_unavailable() {
    let events = vec![UsageEvent::for_test(
        Provider::Claude,
        "session-a",
        "2026-01-01T00:00:00Z",
        42,
    )];
    let idle = compute_provider_summary(
        Provider::Claude,
        &events,
        Some(&SourceHealth::detected(Provider::Claude)),
        "2026-01-01T00:03:00Z",
        "2026-01-01",
    );
    assert_eq!(idle.state, UsageState::Idle);
    assert_eq!(idle.current_session_tokens, Some(42));

    let unavailable = compute_provider_summary(
        Provider::Codex,
        &events,
        Some(&SourceHealth::new(Provider::Codex, "not_detected")),
        "2026-01-01T00:03:00Z",
        "2026-01-01",
    );
    assert_eq!(unavailable.state, UsageState::Unavailable);
    assert_eq!(unavailable.today_tokens, 0);
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml usage::provider_summary`

Expected: FAIL because the Provider-level derivation does not exist.

- [x] **Step 2: Implement the smallest pure Provider derivation.**

Filter only events for the requested Provider. Reuse the existing active-provider timestamp/session rules for the latest session and the two-minute activity window. Sum only events on the supplied Windows local day. Map a detected/limited/malformed Source with no recent event to `Idle`; map not-detected, disabled, invalid-root, permission-denied, and unavailable sources with no events to `Unavailable`. Retain last known session/today values for an idle Provider.

- [x] **Step 3: Extend `UsageSummary` constructors and stale behavior.**

Add fixed Claude/Codex Provider entries to `loading()`, `unavailable()`, and `stale_from()`. Do not remove aggregate fields; stale summaries clone Provider entries as well.

- [x] **Step 4: Wire `compute_summary` to Provider entries.**

Compute both Provider summaries from enabled events, use their sum for aggregate `today_tokens`, and preserve the existing active-provider state/last-update semantics. Keep source health independent and keep disabled-Provider historical events excluded.

- [x] **Step 5: Update Rust integration/privacy tests.**

Add assertions for Claude/Codex totals and aggregate sum to `collection_core.rs` and `runtime_integration.rs`. Update every `UsageSummary` literal to include Provider entries and update serialized allow-list tests to assert no raw fields.

- [x] **Step 6: Run the collection and runtime tests.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml collection:: usage::provider_summary runtime_integration`

Expected: PASS, with no changes to adapter parsing, checkpoints, SQLite schema, or raw-data handling.

---

### Task 3: Persist widget preferences and broadcast updates through Rust

**Files:**
- Modify: `src-tauri/src/database/settings.rs`
- Modify: `src-tauri/src/database/connection.rs`
- Modify: `src-tauri/src/app/runtime.rs`
- Modify: `src-tauri/src/commands/widget_settings.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/database/settings.rs`
- Test: `src-tauri/src/commands/widget_settings.rs`

**Interfaces:**
- `IndexStore::load_widget_settings() -> Result<WidgetSettingsSnapshot, StorageError>`.
- `IndexStore::save_widget_settings(&mut self, &WidgetSettingsSnapshot) -> Result<(), StorageError>`.
- `AppState::widget_settings() -> Result<WidgetSettingsSnapshot, RuntimeError>` and `AppState::update_widget_settings(...)`.
- `get_widget_settings` returns the typed snapshot; `update_widget_settings` validates, persists, updates shared state, emits `widget-settings-changed`, and returns the persisted snapshot.

- [x] **Step 1: Write failing SQLite preference tests.**

Add tests proving defaults, round-trip persistence, and invalid-value fallback using the existing temporary SQLite store:

```rust
#[test]
fn widget_preferences_default_and_round_trip_without_schema_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("index.sqlite");
    let mut store = IndexStore::open(&path).unwrap();
    assert_eq!(store.load_widget_settings().unwrap(), WidgetSettingsSnapshot::defaults());

    let settings = WidgetSettingsSnapshot::new(false, [
        (Provider::Claude, true),
        (Provider::Codex, false),
    ]);
    store.save_widget_settings(&settings).unwrap();
    assert_eq!(store.load_widget_settings().unwrap(), settings);
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml database::settings`

Expected: FAIL because widget settings persistence is not defined.

- [x] **Step 2: Implement key/value persistence with safe defaults.**

Use only `widget.dark_mode`, `widget.visible.claude`, and `widget.visible.codex` in the existing `settings` table. Treat missing values as defaults; treat malformed values as the default for that preference. Do not log or persist arbitrary submitted strings.

- [x] **Step 3: Add AppState accessors and command mapping.**

Keep the snapshot in the existing runtime mutex, persist before replacing shared state, and map storage/runtime failures to `widget_settings_unavailable` or `widget_settings_write`. Use `tauri::Emitter` to emit only the typed snapshot after a successful write.

- [x] **Step 4: Run Rust persistence and command tests.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml database::settings commands::widget_settings app::runtime`

Expected: PASS; existing source-settings commands and live-refresh behavior remain unchanged.

---

### Task 4: Add strict frontend bridges and hooks with tests first

**Files:**
- Create: `src/lib/widget-settings.ts`
- Create: `src/hooks/useUsageSummary.ts`
- Create: `src/hooks/useWidgetSettings.ts`
- Create: `src/tests/lib/widget-settings.test.ts`
- Create: `src/tests/hooks/useUsageSummary.test.tsx`
- Create: `src/tests/hooks/useWidgetSettings.test.tsx`
- Modify: `src/lib/usage-summary.ts`
- Move: `src/lib/usage-summary.test.ts` to `src/tests/lib/usage-summary.test.ts`
- Move: `src/lib/source-settings.test.ts` to `src/tests/lib/source-settings.test.ts`

**Interfaces:**
- `parseUsageSummary` strictly validates the new `providers` collection and existing fields.
- `parseWidgetSettings`, `getWidgetSettings`, `updateWidgetSettings`, and `listenForWidgetSettings` accept only documented Provider IDs, booleans, safe integers, timestamps, and event payloads.
- Hooks subscribe before their initial command, guard unmounted React effects, and clean up Tauri listeners.

- [x] **Step 1: Write failing bridge/hook tests.**

Cover unknown raw fields, duplicate/missing Provider entries, invalid counters, event filtering, command names, default loading, and cleanup. A representative widget-settings parser test is:

```typescript
it("rejects widget settings carrying raw data", () => {
  expect(parseWidgetSettings({
    visibleProviders: [
      { provider: "claude", visible: true },
      { provider: "codex", visible: true },
    ],
    darkMode: true,
    rawRecord: "private text",
  })).toBeNull();
});
```

Run: `npm test -- --run src/tests/lib/widget-settings.test.ts src/tests/hooks`

Expected: FAIL because the bridge and hooks do not exist.

- [x] **Step 2: Implement the strict widget-settings bridge.**

Use `invoke<unknown>` and `listen<unknown>`, exact allowed-key checks, fixed Provider ID validation, and `Error("invalid_widget_settings")` for invalid command results. Emit/listen on the literal `widget-settings-changed` event.

- [x] **Step 3: Extend the usage-summary decoder and relative labels.**

Require exactly one Claude and one Codex Provider entry, validate each bounded field, and retain raw-field rejection. Change the visual relative label helper to return `No updates yet`, `just now`, `N min ago`, `N hr ago`, or `N d ago` so the live overlay matches the reviewed preview; accessible labels continue to include the token unit.

- [x] **Step 4: Implement the two subscription hooks.**

`useUsageSummary` starts with a loading summary, subscribes before `getUsageSummary()`, updates only while mounted, and falls back to unavailable on bridge failure. `useWidgetSettings` starts with both visible/Dark defaults, subscribes before loading, applies valid broadcast snapshots, and cleans up on unmount.

- [x] **Step 5: Move all frontend tests into `src/tests`.**

Keep bridge tests beside the test responsibility rather than beside production modules. Update imports only; do not change test semantics while moving.

- [x] **Step 6: Run the focused frontend tests.**

Run: `npm test -- --run src/tests/lib src/tests/hooks`

Expected: PASS with strict payload rejection and listener cleanup proven.

---

### Task 5: Refactor the overlay into focused React components

**Files:**
- Create: `src/components/widget/TokenTracingWidget.tsx`
- Create: `src/components/widget/WidgetHeader.tsx`
- Create: `src/components/widget/ProviderUsageRow.tsx`
- Create: `src/components/widget/WidgetTotal.tsx`
- Create: `src/components/widget/widget-types.ts`
- Create: `src/tests/components/widget/TokenTracingWidget.test.tsx`
- Delete: `src/App.tsx`
- Delete: `src/App.test.tsx`
- Modify: `src/main.tsx`

**Interfaces:**
- `TokenTracingWidget` consumes `UsageSummary` from `useUsageSummary()` and preferences from `useWidgetSettings()`.
- `WidgetHeader` owns only the draggable, non-interactive header and aggregate status.
- `ProviderUsageRow` receives a bounded `ProviderUsageSummary` plus fixed display metadata and renders `Session`, `Today`, and update text.
- `WidgetTotal` renders the aggregate `Total` value without Provider-specific raw data.

- [x] **Step 1: Write failing component tests against the intended semantic tree.**

Move the existing App behavior tests into `src/tests/components/widget/TokenTracingWidget.test.tsx` and add assertions for both Provider rows, separate numeric values, `Total`, no logo mark/global `Today`, hidden Providers, active/idle state text, and the header drag marker.

```typescript
expect(await screen.findByRole("heading", { name: "Token Tracing" })).toBeInTheDocument();
expect(screen.getByRole("heading", { name: "Claude Code" })).toBeInTheDocument();
expect(screen.getByRole("heading", { name: "Codex" })).toBeInTheDocument();
expect(screen.getByText("Total")).toBeInTheDocument();
expect(screen.queryByText("Total today")).not.toBeInTheDocument();
expect(screen.queryByText("Today", { selector: ".widget__label" })).not.toBeInTheDocument();
```

Run: `npm test -- --run src/tests/components/widget/TokenTracingWidget.test.tsx`

Expected: FAIL because the focused components do not exist.

- [x] **Step 2: Implement the small presentational components.**

Keep Provider display names and colors in a fixed metadata module; do not render Provider-supplied strings as markup. Use semantic headings/labels and `aria-label` values such as `Session: 42,184 tokens` and `Today: 147,271,872 tokens`.

- [x] **Step 3: Implement the widget composition and visibility filter.**

Render both Provider summaries in Claude/Codex order, filter only by persisted `visibleProviders`, preserve a fallback row for loading/unavailable summary states, and keep the header as the only `data-tauri-drag-region` element.

- [x] **Step 4: Update the main entry point.**

Mount `<TokenTracingWidget />` from `src/main.tsx` and import the new style entry. Do not leave a second root App implementation behind.

- [x] **Step 5: Run widget tests and TypeScript.**

Run: `npm test -- --run src/tests/components/widget/TokenTracingWidget.test.tsx`

Expected: PASS with no change to Tauri command/event names.

---

### Task 6: Refactor Settings into focused components while preserving source editing

**Files:**
- Create: `src/components/settings/SettingsScreen.tsx`
- Create: `src/components/settings/ProviderVisibilitySection.tsx`
- Create: `src/components/settings/SourceSettingsSection.tsx`
- Create: `src/components/settings/AppearanceSection.tsx`
- Create: `src/components/settings/SettingsSwitch.tsx`
- Create: `src/components/settings/settings-types.ts`
- Create: `src/tests/components/settings/SettingsScreen.test.tsx`
- Delete: `src/Settings.tsx`
- Delete: `src/Settings.test.tsx`
- Modify: `src/settings-main.tsx`

**Interfaces:**
- `SettingsScreen` loads source settings, widget settings, and the typed usage summary; it owns save/error/loading state.
- `ProviderVisibilitySection` edits only `visibleProviders`.
- `SourceSettingsSection` edits only `enabled`, root text, and progressive `Change…` disclosure; it never sends raw paths outside the existing source-settings command.
- `AppearanceSection` edits only `darkMode`.

- [x] **Step 1: Write failing Settings tests for the reviewed layout and preserved behavior.**

Move the current source-settings tests and add assertions that `Settings`, `Choose what stays visible.`, `Visible providers`, `Sources`, `Appearance`, both Provider names, independent switches, and `Save changes` render. Test that `Change…` reveals the named root input, empty root becomes `null`, both source updates and the widget-settings update are sent, and sanitized errors never echo a submitted path.

Run: `npm test -- --run src/tests/components/settings/SettingsScreen.test.tsx`

Expected: FAIL because the component hierarchy and widget-settings bridge are not wired.

- [x] **Step 2: Implement the Settings form state.**

Use fixed Provider metadata and a local draft. Load all three typed snapshots on mount. Keep automatic root labels `.claude/projects` and `.codex/sessions` for the compact row; show the existing input only after `Change…` is activated. Keep the current generic error mapping and success message.

- [x] **Step 3: Implement theme class synchronization.**

Apply `settings-page--dark` from the local draft so the panel changes immediately when the user toggles Dark mode. After Save, call `updateWidgetSettings`; the command event updates the overlay and any other open Settings view.

- [x] **Step 4: Wire the Settings entry point.**

Mount `<SettingsScreen />` from `src/settings-main.tsx`; keep `settings.html` and the Vite multi-entry configuration intact.

- [x] **Step 5: Run Settings tests.**

Run: `npm test -- --run src/tests/components/settings/SettingsScreen.test.tsx`

Expected: PASS with source persistence and widget-preference updates both covered.

---

### Task 7: Split styles and bind the reviewed Claude editorial system

**Files:**
- Create: `src/styles/tokens.css`
- Create: `src/styles/base.css`
- Create: `src/styles/widget.css`
- Create: `src/styles/settings.css`
- Create: `src/styles/index.css`
- Delete: `src/styles.css`
- Modify: `src/main.tsx`
- Modify: `src/settings-main.tsx`
- Modify: `index.html`
- Modify: `settings.html`

**Interfaces:**
- `tokens.css` owns palette, typography, spacing, radius, and focus variables.
- `base.css` owns reset, document sizing, focus-visible, and reduced-motion rules.
- `widget.css` owns only the 440 × 300 utility tile and its bounded rows.
- `settings.css` owns only the single-panel Settings layout, source disclosure, controls, and light/dark variants.
- `index.css` is the only stylesheet entry imported by both webview entry points.

- [x] **Step 1: Add DOM/style contract tests before deleting the monolith.**

Add a small frontend test that renders the widget and Settings, asserts the expected class hooks (`widget`, `widget__header`, `settings-page`, `settings-page--dark`), and verifies the header has the drag marker. Visual values are verified by browser preview and build; do not add a CSS testing dependency.

- [x] **Step 2: Split the existing global declarations into focused files.**

Move tokens/reset first, then widget rules, then Settings rules. Remove duplicated selectors and do not change behavior during the move. Keep the public class names emitted by the new components stable.

- [x] **Step 3: Bind the approved Claude editorial tokens.**

Use `#faf9f5` cream, `#181715` dark, `#252320` raised dark, `#cc785c` coral, `#5db8a6` health, near-black ink, hairlines, Copernicus/Tiempos/Cormorant serif fallback, and StyreneB/Inter/system sans fallback. Keep Provider dots as the only Provider-specific accents. Use no gradients, card shadows, logos, or decorative animation.

- [x] **Step 4: Fit the two surfaces at their actual windows.**

Keep the overlay readable at 440 × 300 logical pixels with bounded Provider names, numeric values, and update labels. Keep Settings keyboard-usable at 520 × 560 and allow root disclosure to scroll rather than clip. Use 44px controls in Settings, visible Focus Blue rings, and `prefers-reduced-motion` coverage.

- [x] **Step 5: Run the focused frontend suite and build.**

Run: `npm test -- --run`

Expected: all frontend tests pass with no warnings.

Run: `npm run build`

Expected: both `dist/index.html` and `dist/settings.html` emit successfully.

---

### Task 8: Resize the main window and verify end-to-end behavior

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Review: `src-tauri/src/app/window.rs`, `src-tauri/src/app/tray.rs`, `src-tauri/capabilities/default.json`
- Test: `src-tauri/tests/runtime_integration.rs`, `src-tauri/src/app/window.rs`, `src-tauri/src/app/tray.rs`

**Interfaces:**
- Main window becomes approximately `440 × 300`, remains frameless, transparent, non-resizable, non-topmost, and taskbar-hidden.
- Existing close-to-hide and tray Settings behavior remain unchanged.

- [x] **Step 1: Write/update failing configuration assertions.**

Add a focused test or JSON inspection command that expects the main window dimensions to be `440` × `260`, `alwaysOnTop: false`, `decorations: false`, `transparent: true`, and `skipTaskbar: true` before changing the configuration.

- [x] **Step 2: Change only the approved main-window dimensions.**

Modify `src-tauri/tauri.conf.json` from the current overlay size to 440 × 300. Preserve all other existing flags and do not add an Always on top setting.

- [x] **Step 3: Run Rust, frontend, and integrated checks.**

Run:

```powershell
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug
```

Expected: every command exits zero; the debug executable contains both frontend entries and no new network/filesystem permission.

- [ ] **Step 4: Run the Windows 11 smoke pass.**

**Status:** Not run in this environment. The debug executable was built and
the automated Rust/frontend/integrated checks are green; native interaction,
tray behavior, foreground-window coverage, and 125%/150% scaling still need
an actual Windows 11 desktop pass.

Against the debug executable, verify: both Provider rows update from real local metadata; a Provider can be hidden without affecting collection; Source toggles still refresh collection; Dark mode updates Settings and overlay after Save; the header drags while body/footer do not; the overlay is covered by a foreground window; close hides instead of quitting; tray Show/Hide/Settings/Quit remain correct; and 125%/150% scaling has no clipping.

- [x] **Step 5: Run privacy and diff checks.**

Inspect serialized summary/widget-settings tests, bridge allow-lists, schema tests, and `git diff --check`. Confirm no prompt, response, reasoning, tool payload, credential, repository content, working directory, raw record, absolute root, network client, or frontend state library was introduced.

---

## Plan self-review

### Spec coverage

- Reviewed variant D visual hierarchy, typography, colors, spacing, no-logo/no-global-Today decisions: Tasks 5–7.
- Independent Claude/Codex Session and Today totals plus aggregate Total: Tasks 2 and 5.
- Visible-Provider preferences and synchronized Dark mode: Tasks 1, 3, 4, and 6.
- Existing source settings, validation, watcher refresh, tray, close-to-hide, and privacy contracts: Tasks 3, 6, and 8.
- Frontend folder organization with focused ownership and tests separated under `src/tests`: Tasks 4–7.
- Full frontend/Rust/integrated verification: Task 8.

### Placeholder scan

The plan names every file, command, interface, expected failure/pass condition,
and implementation boundary. It contains no `TBD`, `TODO`, or unspecified
follow-up task.

### Type consistency

`ProviderUsageSummary` is the Rust/TypeScript per-Provider wire item;
`WidgetSettingsSnapshot` is the Rust/TypeScript preference snapshot;
`widget-settings-changed` is the single preference event literal; existing
source-settings and usage-summary command/event literals remain unchanged.
