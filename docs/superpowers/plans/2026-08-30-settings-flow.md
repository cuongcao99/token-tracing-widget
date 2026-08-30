# Settings Flow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a minimal settings window that lets users enable or disable Claude Code and Codex sources and replace their explicitly configured roots, then refreshes live collection without exposing raw session data.

**Architecture:** Rust owns the typed settings command boundary, SQLite persistence, validation, and watcher refresh. A dynamically created Tauri settings window loads a separate React entry point; the React bridge strictly parses the settings snapshot and sends only provider, enabled state, and optional root override. The existing path-free `ConfigurationChanged` signal causes the live collector to rebuild enabled watch roots and recollect.

**Tech Stack:** Rust, Tauri 2 commands and webview windows, SQLite through the existing `IndexStore`, React 19, TypeScript, Vite, Vitest, Testing Library, plain CSS.

**Spec:** `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`, with source configuration details in `docs/superpowers/specs/2026-08-30-source-configuration-wsl-design.md`.

## Global Constraints

- Keep version one local-only and Windows 11-only.
- Keep filesystem, collection, and SQLite access in Rust. The React webview receives typed summaries, plus configured source roots only in settings flows.
- Preserve metadata-only collection: prompts, responses, reasoning, tool payloads, credentials, repository contents, and working directories never enter normalized events, SQLite, diagnostics, or frontend payloads.
- Keep provider-specific formats behind adapters and enforce normalization, delta conversion, deduplication, and checkpoint invariants in the collection core.
- Add no network client, telemetry, sidecar, background service, frontend state library, CSS framework, or ORM without an approved design change.
- Use `dev` for ongoing work and do not merge or push `main` from this slice.
- Treat blank root input as automatic native-path selection; non-blank input must pass the existing explicit-root validator.
- Command errors contain only stable sanitized categories and never echo a submitted path.
- No overlay redesign, always-below behavior, launch-on-login, opacity, position reset, clear-index action, or arbitrary file picker is part of this slice.

---

### Task 1: Add the typed Rust settings command contract

**Files:**
- Create: `src-tauri/src/commands/source_settings.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/source_settings.rs` (unit tests)

**Interfaces:**
- Consumes: `AppState::source_config`, `SourceConfig::try_new`, `parse_explicit_root`, and `update_source_config_and_refresh`.
- Produces:
  - `get_source_settings` Tauri command with JSON result `{ "sources": [{ "provider": "claude|codex", "enabled": boolean, "rootOverride": string|null }] }`.
  - `update_source_settings` Tauri command accepting `{ "settings": { "provider": "claude|codex", "enabled": boolean, "rootOverride": string|null } }` and returning the same snapshot.
  - `SourceSettingsInput`, `SourceSettingsView`, and `SourceSettingsSnapshot` as the typed boundary types.

- [ ] **Step 1: Write the failing command-contract tests**

Add tests proving the wire shape and privacy boundary before adding the commands:

```rust
#[test]
fn settings_snapshot_contains_only_allowed_fields() {
    let snapshot = SourceSettingsSnapshot {
        sources: vec![SourceSettingsView {
            provider: Provider::Claude,
            enabled: true,
            root_override: Some(r"\\wsl.localhost\Ubuntu\home\user\.claude\projects".to_owned()),
        }],
    };
    let object = serde_json::to_value(snapshot)
        .expect("settings should serialize")
        .as_object()
        .cloned()
        .expect("settings should be an object");

    assert_eq!(object.keys().map(String::as_str).collect::<Vec<_>>(), ["sources"]);
    let source = object["sources"][0].as_object().unwrap();
    assert_eq!(
        source.keys().map(String::as_str).collect::<Vec<_>>(),
        ["enabled", "provider", "rootOverride"]
    );
    assert!(!serde_json::to_string(&object).unwrap().contains("profileRoot"));
    assert!(!serde_json::to_string(&object).unwrap().contains("rawRecord"));
}

#[test]
fn input_rejects_unknown_raw_data_fields() {
    let value = serde_json::json!({
        "provider": "claude",
        "enabled": true,
        "rootOverride": null,
        "prompt": "private text"
    });

    assert!(serde_json::from_value::<SourceSettingsInput>(value).is_err());
}

#[test]
fn blank_root_override_becomes_automatic_without_echoing_input() {
    let input = SourceSettingsInput {
        provider: Provider::Codex,
        enabled: true,
        root_override: Some("  ".to_owned()),
    };

    let config = input.into_config().expect("blank should mean automatic");
    assert!(config.root_override().is_none());
}

#[test]
fn invalid_root_error_does_not_include_submitted_path() {
    let submitted = r"\\server\private\sessions";
    let input = SourceSettingsInput {
        provider: Provider::Claude,
        enabled: true,
        root_override: Some(submitted.to_owned()),
    };

    let error = input.into_config().unwrap_err();
    assert!(!error.contains(submitted));
    assert_eq!(error, "invalid_root:unsupported_unc");
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::source_settings`

Expected: FAIL because the command boundary types and conversion helper do not exist yet.

- [ ] **Step 2: Run the focused test and confirm the expected RED failure**

Run the command above and verify the failure is missing production symbols, not a malformed test or privacy assertion.

- [ ] **Step 3: Implement the minimal sanitized command boundary**

Define the public command DTOs with `serde(rename_all = "camelCase")` and `serde(deny_unknown_fields)` on the input. Convert blank strings to `None`, call `parse_explicit_root` for non-blank strings, and map all runtime failures to stable categories such as `settings_unavailable`, `settings_write`, or `settings_refresh`.

The command flow must be:

```rust
let config = settings.into_config()?;
update_source_config_and_refresh(state.inner(), live_handle.inner(), config)
    .map_err(sanitize_runtime_error)?;
source_settings_snapshot(state.inner())
```

Build the snapshot in the fixed order Claude then Codex, and expose only `rootOverride`, never `configured_root_label` or a profile path.

- [ ] **Step 4: Register the module and commands**

Export `source_settings` from `commands/mod.rs` and add both commands to the `tauri::generate_handler!` list in `src-tauri/src/lib.rs`.

- [ ] **Step 5: Run the focused tests and Rust check**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::source_settings`

Expected: PASS, including the unknown-field and path-redaction assertions.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS with both commands registered.

- [ ] **Step 6: Commit the command boundary**

```bash
git add src-tauri/src/commands src-tauri/src/lib.rs
git commit -m "feat: expose typed source settings commands"
```

### Task 2: Add the strict TypeScript settings bridge

**Files:**
- Create: `src/lib/source-settings.ts`
- Create: `src/lib/source-settings.test.ts`

**Interfaces:**
- Consumes: Tauri `get_source_settings` and `update_source_settings` commands.
- Produces: `ProviderId`, `SourceSettings`, `SourceSettingsSnapshot`, `getSourceSettings()`, `updateSourceSettings()`, and `parseSourceSettings()` for the settings screen.

- [ ] **Step 1: Write failing bridge tests**

Add tests for strict parsing, command names/arguments, and forbidden fields:

```typescript
it("requests and validates the typed settings snapshot", async () => {
  vi.mocked(invoke).mockResolvedValue({
    sources: [
      { provider: "claude", enabled: true, rootOverride: null },
      { provider: "codex", enabled: false, rootOverride: "C:\\codex" },
    ],
  });

  await expect(getSourceSettings()).resolves.toEqual({
    sources: [
      { provider: "claude", enabled: true, rootOverride: null },
      { provider: "codex", enabled: false, rootOverride: "C:\\codex" },
    ],
  });
  expect(invoke).toHaveBeenCalledWith("get_source_settings");
});

it("sends only the source settings payload", async () => {
  vi.mocked(invoke).mockResolvedValue({
    sources: [
      { provider: "claude", enabled: false, rootOverride: null },
      { provider: "codex", enabled: true, rootOverride: null },
    ],
  });

  await updateSourceSettings({
    provider: "claude",
    enabled: false,
    rootOverride: null,
  });

  expect(invoke).toHaveBeenCalledWith("update_source_settings", {
    settings: { provider: "claude", enabled: false, rootOverride: null },
  });
});

it("rejects a settings payload containing raw session data", () => {
  expect(
    parseSourceSettings({
      sources: [{ provider: "claude", enabled: true, rootOverride: null, prompt: "secret" }],
    }),
  ).toBeNull();
});
```

Run: `npm test -- --run src/lib/source-settings.test.ts`

Expected: FAIL because the bridge module does not exist.

- [ ] **Step 2: Implement strict parsing and command wrappers**

Accept exactly two unique provider records, the two supported provider IDs, booleans for `enabled`, and only string-or-null `rootOverride`. Reject unknown keys, arrays in place of objects, duplicate providers, and snapshots that omit either supported provider. Parse the command result before returning it. Pass the nested `settings` argument exactly as shown above.

- [ ] **Step 3: Run bridge tests and the existing frontend suite**

Run: `npm test -- --run src/lib/source-settings.test.ts src/lib/usage-summary.test.ts`

Expected: PASS with no raw-field payload accepted.

- [ ] **Step 4: Commit the bridge**

```bash
git add src/lib/source-settings.ts src/lib/source-settings.test.ts
git commit -m "feat: add typed source settings bridge"
```

### Task 3: Build the minimal React settings screen

**Files:**
- Create: `src/Settings.tsx`
- Create: `src/Settings.test.tsx`
- Create: `src/settings-main.tsx`
- Create: `settings.html`
- Modify: `src/styles.css`

**Interfaces:**
- Consumes: `getSourceSettings()` and `updateSourceSettings()` from `src/lib/source-settings.ts`.
- Produces: a settings page with one card per provider, enable toggles, optional root override inputs, loading/error/saving states, and a save confirmation.

- [ ] **Step 1: Write failing component tests**

Mock only the typed bridge boundary and test user-visible behavior:

```typescript
it("renders both persisted provider settings", async () => {
  vi.mocked(getSourceSettings).mockResolvedValue({
    sources: [
      { provider: "claude", enabled: true, rootOverride: null },
      { provider: "codex", enabled: false, rootOverride: "C:\\codex" },
    ],
  });

  render(<Settings />);

  expect(await screen.findByRole("heading", { name: "Source settings" })).toBeInTheDocument();
  expect(screen.getByRole("checkbox", { name: "Collect Claude Code" })).toBeChecked();
  expect(screen.getByRole("checkbox", { name: "Collect Codex" })).not.toBeChecked();
  expect(screen.getByRole("textbox", { name: "Codex source root" })).toHaveValue("C:\\codex");
});

it("saves the changed provider settings and reports success", async () => {
  vi.mocked(getSourceSettings).mockResolvedValue({
    sources: [
      { provider: "claude", enabled: true, rootOverride: null },
      { provider: "codex", enabled: true, rootOverride: null },
    ],
  });
  vi.mocked(updateSourceSettings).mockResolvedValue({
    sources: [
      { provider: "claude", enabled: false, rootOverride: null },
      { provider: "codex", enabled: true, rootOverride: null },
    ],
  });

  render(<Settings />);
  await screen.findByRole("heading", { name: "Source settings" });
  fireEvent.click(screen.getByRole("checkbox", { name: "Collect Claude Code" }));
  fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

  await waitFor(() => expect(updateSourceSettings).toHaveBeenCalled());
  expect(updateSourceSettings).toHaveBeenCalledWith({
    provider: "claude",
    enabled: false,
    rootOverride: null,
  });
  expect(await screen.findByRole("status")).toHaveTextContent("Saved");
});

it("shows a sanitized error when saving is rejected", async () => {
  vi.mocked(getSourceSettings).mockResolvedValue({
    sources: [
      { provider: "claude", enabled: true, rootOverride: null },
      { provider: "codex", enabled: true, rootOverride: null },
    ],
  });
  vi.mocked(updateSourceSettings).mockRejectedValue(new Error("invalid_root:unsupported_unc"));

  render(<Settings />);
  await screen.findByRole("heading", { name: "Source settings" });
  fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

  expect(await screen.findByRole("alert")).toHaveTextContent("Invalid source root");
  expect(screen.getByRole("alert")).not.toHaveTextContent("\\server");
});
```

Run: `npm test -- --run src/Settings.test.tsx`

Expected: FAIL because the screen and bridge imports do not exist.

- [ ] **Step 2: Implement the form state and provider cards**

Render fixed display names for `claude` and `codex`. Keep a local form copy of `enabled` and a string root input; map an empty input back to `rootOverride: null`. Load once on mount. On submit, update Claude then Codex sequentially through the bridge, replace the local snapshot with each returned result, and show `Saved. Collection will refresh shortly.` only after all updates succeed.

- [ ] **Step 3: Add privacy-safe loading and error handling**

Map only known error categories (`invalid_root:*`, `settings_write`, `settings_refresh`, and `settings_unavailable`) to user-facing generic text. Use a generic fallback for all other errors, and never render the exception string or a submitted path.

- [ ] **Step 4: Add the settings entry point and scoped plain CSS**

Create `settings.html` pointing to `/src/settings-main.tsx`; mount `<Settings />` from that entry and import the existing stylesheet. Add only `.settings-page`, `.source-card`, form-control, status, and button styles. Keep the existing overlay styles and dimensions unchanged.

- [ ] **Step 5: Run component tests and build**

Run: `npm test -- --run src/Settings.test.tsx src/App.test.tsx`

Expected: PASS.

Run: `npm run build`

Expected: PASS and `dist/settings.html` is emitted alongside the main entry.

- [ ] **Step 6: Commit the settings screen**

```bash
git add src/Settings.tsx src/Settings.test.tsx src/settings-main.tsx settings.html src/styles.css
git commit -m "feat: add source settings screen"
```

### Task 4: Open the settings screen from the tray

**Files:**
- Modify: `src-tauri/src/app/tray.rs`
- Modify: `src-tauri/src/app/window.rs` only if the settings window event behavior needs a focused regression test
- Modify: `src-tauri/tauri.conf.json` only if the dynamic webview requires an app-level setting
- Modify: `src-tauri/capabilities/default.json`
- Test: `src-tauri/src/app/tray.rs` (unit tests)

**Interfaces:**
- Consumes: `settings.html` emitted by Vite and the registered settings commands.
- Produces: a `Settings` tray action that creates one decorated, non-topmost settings window on demand, focuses an existing settings window, and lets closing it destroy the window.

- [ ] **Step 1: Write failing tray tests**

Extend the existing tray tests:

```rust
#[test]
fn settings_menu_id_opens_settings_window() {
    assert_eq!(action_for_menu_id(SETTINGS_MENU_ID), TrayAction::Settings);
}

#[test]
fn tray_menu_items_include_settings_before_quit() {
    assert_eq!(
        menu_items(),
        [
            (SHOW_MENU_ID, "Show"),
            (HIDE_MENU_ID, "Hide"),
            (SETTINGS_MENU_ID, "Settings"),
            (QUIT_MENU_ID, "Quit"),
        ]
    );
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml app::tray`

Expected: FAIL because `SETTINGS_MENU_ID` and `TrayAction::Settings` do not exist.

- [ ] **Step 2: Add the settings tray action and dynamic window builder**

Add `SETTINGS_WINDOW_LABEL = "settings"` and `SETTINGS_MENU_ID = "settings"`. Use `WebviewWindowBuilder::new(app, SETTINGS_WINDOW_LABEL, WebviewUrl::App("settings.html".into()))` with a normal decorated window around `520x560`, resizable, and not always-on-top. If the window already exists, call `show()` and `set_focus()` instead of creating a second one. Keep the main overlay visibility actions unchanged.

- [ ] **Step 3: Add settings to the capability window scope**

Change `windows` in `src-tauri/capabilities/default.json` from `["main"]` to `["main", "settings"]`; keep permissions limited to the existing core defaults and drag permission. Do not add filesystem or network permissions.

- [ ] **Step 4: Run tray tests and the integrated Tauri build**

Run: `cargo test --manifest-path src-tauri/Cargo.toml app::tray`

Expected: PASS.

Run: `npm run tauri build -- --debug`

Expected: PASS and the debug executable includes the settings entry.

- [ ] **Step 5: Commit tray and capability wiring**

```bash
git add src-tauri/src/app/tray.rs src-tauri/capabilities/default.json
git commit -m "feat: open settings from system tray"
```

### Task 5: Run cross-boundary privacy and regression verification

**Files:**
- Modify: `src-tauri/tests/runtime_integration.rs` only if a command-boundary integration test needs the existing runtime fixture helpers
- Modify: `src/lib/source-settings.test.ts` or `src/Settings.test.tsx` only for regression cases discovered by the focused runs
- Modify: `docs/superpowers/plans/2026-08-30-settings-flow.md` to mark completed steps

**Interfaces:**
- Consumes: all command, bridge, UI, tray, runtime, persistence, and watcher work from Tasks 1–4.
- Produces: verified settings persistence, sanitized frontend payloads, independent provider updates, and a clean branch ready for review.

- [ ] **Step 1: Add the narrowest runtime regression test for update ordering**

Prove that updating a valid source config persists it and changes the in-memory config used by the next collection, while a failed invalid input never reaches SQLite. Reuse `AppState::from_paths` and `IndexStore::load_source_configs`; do not inspect raw source files in the UI test.

- [ ] **Step 2: Run all required gates**

Run each command separately:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm test -- --run
npm run build
npm run tauri build -- --debug
```

Expected: every command exits zero; Rust tests cover source config, persistence, runtime, collection, watcher refresh, and commands; frontend tests cover summary and settings payload rejection.

- [ ] **Step 3: Inspect the final diff for scope and privacy**

Run:

```bash
git diff --check HEAD~5..HEAD
git status --short --branch
```

Confirm the diff contains no raw fixtures, prompts, responses, repository paths, new permissions, network code, or unrelated overlay redesign.

- [ ] **Step 4: Commit final test-only adjustments and plan progress**

```bash
git add docs/superpowers/plans/2026-08-30-settings-flow.md src-tauri/tests src/lib
git commit -m "test: verify source settings boundary"
```
