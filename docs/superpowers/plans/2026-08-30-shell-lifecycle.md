# Shell Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax.

**Goal:** Give the Windows overlay a reliable tray lifecycle: Show, Hide, Quit, and close-to-hide while keeping the live collector running until Quit or process exit.

**Architecture:** Keep shell behavior in the existing Rust seams. tray.rs owns stable menu IDs, the tray icon, menu dispatch, and main-window visibility; window.rs owns the CloseRequested policy; lib.rs wires both into the existing Tauri builder and preserves the live collector shutdown callback. React, the summary contract, collection, and storage remain unchanged.

**Tech Stack:** Rust 2021, Tauri 2 with its existing tray-icon feature, the Windows desktop runtime, checked-in icons/icon.ico, React 19, TypeScript, Vite, and plain CSS.

**Spec:** docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md

## Global Constraints

- Version 1 remains local-only and Windows 11-only.
- Rust owns filesystem, collection, and SQLite access; React receives typed summaries only.
- Prompts, responses, reasoning, tool payloads, credentials, repository contents, working directories, and raw source data never enter normalized events, SQLite, diagnostics, or frontend payloads.
- Preserve existing provider adapters, incremental checkpoints, delta conversion, deduplication, transactional persistence, and post-commit UsageSummary behavior.
- Do not add a network client, telemetry, sidecar, background service, frontend state library, CSS framework, ORM, watcher crate, or new runtime dependency.
- Enable only Tauri's built-in tray-icon feature; do not add a separate tray package.
- The tray menu in this slice contains exactly Show, Hide, and Quit. Settings is a later slice.
- Closing the main window hides it and prevents process termination. Tray Quit requests normal Tauri exit so RunEvent::Exit shuts down the live collector.
- Keep the existing transparent, frameless, always-on-top, non-taskbar window configuration unchanged.
- Keep usage-summary-changed and get_usage_summary unchanged.
- Work on feat/shell-lifecycle; do not merge or push main.
- Do not commit .claude/ settings or generated provider/session data.

---

## Scope and baseline

The branch starts from the clean, synced dev tip 7f23d4a. The live-collection implementation has automated Rust/frontend/build gates; its remaining acceptance gap is a real Windows 11 notification smoke check. Task 4 includes that check after the shell changes, but this plan does not create a second collector or alter collection behavior.

This plan implements only tray lifecycle and close-to-hide. It explicitly defers Settings, startup registration, single-instance enforcement, remembered position, opacity, configurable always-on-top, explicit WSL root selection, clear-index recovery, installer, and uninstall.

## File map

- Modify src-tauri/Cargo.toml: enable Tauri's built-in tray-icon feature.
- Modify src-tauri/src/app/tray.rs: stable IDs, pure action mapping, menu/icon construction, Show/Hide dispatch, and normal Quit.
- Modify src-tauri/src/app/window.rs: main-window close policy and CloseRequested interception.
- Modify src-tauri/src/lib.rs: register the window handler and create the tray during setup without changing collection order or exit shutdown.
- Test src-tauri/src/app/tray.rs: menu/action contract without a GUI runtime.
- Test src-tauri/src/app/window.rs: close policy for main and unrelated labels.
- Review unchanged src-tauri/tauri.conf.json, src-tauri/capabilities/default.json, src/App.tsx, src/lib/usage-summary.ts, and collection/provider/database/source modules.

## Interfaces

The shell exposes only Rust-private seams:

~~~rust
pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
pub(crate) const TRAY_ID: &str = "main-tray";
pub(crate) const SHOW_MENU_ID: &str = "show";
pub(crate) const HIDE_MENU_ID: &str = "hide";
pub(crate) const QUIT_MENU_ID: &str = "quit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayAction {
    Show,
    Hide,
    Quit,
    Ignore,
}

pub(crate) fn action_for_menu_id(menu_id: &str) -> TrayAction;
pub(crate) fn setup_tray<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), Box<dyn std::error::Error>>;
pub(crate) fn handle_window_event<R: tauri::Runtime>(
    window: &tauri::Window<R>,
    event: &tauri::WindowEvent,
);
~~~

TrayAction::Ignore is the safe default for unknown IDs. No shell event forwards a source path, provider record, summary payload, or other private data.

### Task 1: Establish the tray feature and pure action contract

Files:
- Modify: src-tauri/Cargo.toml
- Modify: src-tauri/src/app/tray.rs
- Test: src-tauri/src/app/tray.rs in a cfg(test) module

Interfaces:
- Consumes: the existing Tauri 2 dependency and empty tray.rs.
- Produces: stable IDs, TrayAction, and action_for_menu_id.

- [x] Step 1: Enable the existing Tauri tray feature.

Change only:

~~~toml
tauri = { version = "2", features = ["tray-icon"] }
~~~

Do not add a separate tray dependency, edit Cargo.lock manually, or alter capabilities.

- [x] Step 2: Write failing action-mapping tests.

Add before the implementation:

~~~rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_menu_ids_map_to_exact_actions() {
        assert_eq!(action_for_menu_id(SHOW_MENU_ID), TrayAction::Show);
        assert_eq!(action_for_menu_id(HIDE_MENU_ID), TrayAction::Hide);
        assert_eq!(action_for_menu_id(QUIT_MENU_ID), TrayAction::Quit);
    }

    #[test]
    fn unknown_menu_ids_are_ignored() {
        assert_eq!(action_for_menu_id("settings"), TrayAction::Ignore);
        assert_eq!(action_for_menu_id(""), TrayAction::Ignore);
    }

    #[test]
    fn lifecycle_menu_ids_are_distinct() {
        assert_ne!(SHOW_MENU_ID, HIDE_MENU_ID);
        assert_ne!(SHOW_MENU_ID, QUIT_MENU_ID);
        assert_ne!(HIDE_MENU_ID, QUIT_MENU_ID);
    }
}
~~~

- [x] Step 3: Run the focused test and verify the seam is absent.

Run:

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib app::tray --offline
~~~

Expected: compilation fails because the constants, enum, and matcher do not exist. Do not implement GUI behavior in response to this failure.

- [x] Step 4: Implement the minimal pure contract.

Add the constants, enum, and exact matcher from the Interfaces section. The matcher returns TrayAction::Ignore for every string other than show, hide, or quit.

- [x] Step 5: Run focused tests and formatting.

Run:

~~~powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib app::tray --offline
~~~

Expected: all three tests pass and formatting is clean.

- [x] Step 6: Commit the pure contract.

~~~powershell
git add src-tauri/Cargo.toml src-tauri/src/app/tray.rs
git commit -m "feat: define tray lifecycle contract"
~~~

### Task 2: Build the tray icon and lifecycle handlers

Files:
- Modify: src-tauri/src/app/tray.rs
- Test: src-tauri/src/app/tray.rs in the existing cfg(test) module

Interfaces:
- Consumes: Task 1's IDs and action matcher, the existing main window, and Tauri's MenuBuilder/TrayIconBuilder.
- Produces: setup_tray, one tray icon, and Show/Hide/Quit dispatch.

- [x] Step 1: Write the failing menu-order test.

Add before the menu-definition helper:

~~~rust
#[test]
fn lifecycle_menu_items_are_show_hide_quit_in_order() {
    assert_eq!(
        menu_items(),
        [
            (SHOW_MENU_ID, "Show"),
            (HIDE_MENU_ID, "Hide"),
            (QUIT_MENU_ID, "Quit"),
        ]
    );
}
~~~

Expected: compilation fails because menu_items does not exist.

- [x] Step 2: Implement the pure menu definition.

Add:

~~~rust
fn menu_items() -> [(&'static str, &'static str); 3] {
    [
        (SHOW_MENU_ID, "Show"),
        (HIDE_MENU_ID, "Hide"),
        (QUIT_MENU_ID, "Quit"),
    ]
}
~~~

Use the same constants when constructing the Tauri menu so the test and runtime cannot drift.

- [x] Step 3: Implement safe main-window visibility dispatch.

Import tauri::Manager and add:

~~~rust
fn set_main_window_visibility<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    visible: bool,
) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    let result = if visible { window.show() } else { window.hide() };
    if let Err(error) = result {
        eprintln!("shell:window_visibility:{error}");
    }
}
~~~

Unknown IDs do nothing. Quit must call app.exit(0), never std::process::exit, so the existing RunEvent::Exit callback can shut down LiveCollectionHandle.

- [x] Step 4: Implement setup_tray with the existing default icon.

Construct the menu, require the generated default window icon, and build one tray icon:

~~~rust
pub(crate) fn setup_tray<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    let menu = tauri::menu::MenuBuilder::new(app)
        .text(SHOW_MENU_ID, "Show")
        .text(HIDE_MENU_ID, "Hide")
        .separator()
        .text(QUIT_MENU_ID, "Quit")
        .build()?;
    let icon = app.default_window_icon().cloned().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "default window icon unavailable",
        )
    })?;

    tauri::tray::TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .tooltip("Token Tracing")
        .on_menu_event(|app, event| match action_for_menu_id(event.id().as_ref()) {
            TrayAction::Show => set_main_window_visibility(app, true),
            TrayAction::Hide => set_main_window_visibility(app, false),
            TrayAction::Quit => app.exit(0),
            TrayAction::Ignore => {}
        })
        .build(app)?;

    Ok(())
}
~~~

No capability or frontend API change is part of this task.

- [x] Step 5: Run focused tests and the Rust compile check.

Run:

~~~powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib app::tray --offline
cargo check --manifest-path src-tauri/Cargo.toml --offline
~~~

Expected: action/menu tests pass and Tauri tray APIs compile on Windows.

- [x] Step 6: Commit the tray implementation.

~~~powershell
git add src-tauri/src/app/tray.rs
git commit -m "feat: add tray lifecycle actions"
~~~

### Task 3: Add close-to-hide and wire the shell

Files:
- Modify: src-tauri/src/app/window.rs
- Modify: src-tauri/src/lib.rs
- Test: src-tauri/src/app/window.rs in a cfg(test) module

Interfaces:
- Consumes: MAIN_WINDOW_LABEL, WindowEvent::CloseRequested, setup_tray, and the current live collector setup/shutdown.
- Produces: a handler that hides only main and preserves normal application exit.

- [x] Step 1: Write the failing close-policy test.

Add:

~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tray::MAIN_WINDOW_LABEL;

    #[test]
    fn only_main_window_uses_close_to_hide() {
        assert!(should_hide_on_close(MAIN_WINDOW_LABEL));
        assert!(!should_hide_on_close("settings"));
    }
}
~~~

Expected: compilation fails because should_hide_on_close does not exist.

- [x] Step 2: Implement the close handler.

Add:

~~~rust
use crate::app::tray::MAIN_WINDOW_LABEL;

fn should_hide_on_close(window_label: &str) -> bool {
    window_label == MAIN_WINDOW_LABEL
}

pub(crate) fn handle_window_event<R: tauri::Runtime>(
    window: &tauri::Window<R>,
    event: &tauri::WindowEvent,
) {
    if !should_hide_on_close(window.label()) {
        return;
    }

    if let tauri::WindowEvent::CloseRequested { api } = event {
        api.prevent_close();
        if let Err(error) = window.hide() {
            eprintln!("shell:close_to_hide:{error}");
        }
    }
}
~~~

Do not call window.close() from this handler. Prevent close before hiding. Keep the handler path-free and payload-free.

- [x] Step 3: Wire both shell seams into lib.rs.

Add the global handler before setup:

~~~rust
tauri::Builder::default()
    .on_window_event(app::window::handle_window_event)
    .setup(|app| {
        app::tray::setup_tray(app.handle())?;
        let state = app::runtime::initialize_from_app(app.handle());
~~~

Keep the existing setup operations after the shown insertion: manage AppState, collect and emit the initial post-commit summary, start and manage LiveCollectionHandle, register get_usage_summary, and shut down the live handle on RunEvent::Exit.

Do not intercept ExitRequested, add a Settings menu item, change close behavior for another window, or move filesystem/SQLite access into the frontend.

- [x] Step 4: Run focused and complete Rust checks.

~~~powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib app::window --offline
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
cargo check --manifest-path src-tauri/Cargo.toml --offline
~~~

Expected: close-policy, collection, live-loop, and privacy tests pass.

Observed on Windows: the two pre-existing native-watcher tests can intermittently miss an immediate post-start write when tests run in parallel because `FileWatcher::start` returns before its worker has armed `ReadDirectoryChangesW`. The serialized all-targets run passes; watcher redesign remains outside this shell task's scope.

- [ ] Step 5: Commit the shell wiring.

~~~powershell
git add src-tauri/src/app/window.rs src-tauri/src/lib.rs
git commit -m "feat: hide overlay instead of closing"
~~~

### Task 4: Full verification and live acceptance

Files:
- Review: src-tauri/src/app/tray.rs, src-tauri/src/app/window.rs, src-tauri/src/lib.rs
- Verify unchanged: src-tauri/tauri.conf.json, src-tauri/capabilities/default.json, src/App.tsx, src/lib/usage-summary.ts, collection/provider/database/source modules

Interfaces:
- The release remains one Tauri executable with one in-process live collector and one tray icon.
- The only live event payload remains UsageSummary under usage-summary-changed.
- Close-to-hide keeps the collector alive; Quit reaches the existing exit shutdown.

- [x] Step 1: Run all automated gates.

~~~powershell
git diff --check
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
cargo check --manifest-path src-tauri/Cargo.toml --offline
~~~

Expected: frontend tests/build, Rust tests/check, and formatting pass.

- [x] Step 2: Build the release executable.

~~~powershell
npm run tauri build -- --no-bundle
~~~

Expected: src-tauri/target/release/token-tracing-widget.exe exists, remains a GUI executable, and has no app-managed sidecar or service.

- [ ] Step 3: Perform the Windows 11 shell smoke test.

Launch the release executable and verify the overlay and tray icon appear; Hide hides while the process/live collector remain alive; Show restores; the close button and Alt+F4 hide instead of terminating; Quit terminates and reaches live-handle shutdown; no Settings item or extra window exists; and no console, raw source content, prompt, response, reasoning, tool payload, credential, repository path, working directory, or absolute source path appears.

Verified: rebuilt release launches, the overlay renders, summary updates, and Alt+F4 removes the window while the widget process remains alive. Pending: the available Windows automation surface does not expose the system tray, so Show/Hide/normal tray Quit and the tray-icon visual check could not be exercised here.

- [x] Step 4: Re-run the previously blocked live-collection smoke checks.

Using only metadata-only synthetic data if a fixture is needed, verify a valid append updates without manual refresh, burst writes coalesce, partial final lines wait for completion, 30-second reconciliation repairs a missed notification, an unavailable provider does not stop the other, restart does not duplicate totals, and the process exits without a remaining watcher thread/handle.

Automated live-loop, watcher, runtime-integration, restart/deduplication, reconciliation, partial-line, provider-independence, shutdown, and privacy checks pass. The separate real Windows notification smoke remains the pre-existing acceptance gap noted above.

- [x] Step 5: Review scope and leave the branch ready.

Run:

~~~powershell
git status --short --branch
git diff --name-only dev..HEAD
git diff -- src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/capabilities/default.json
~~~

Expected tracked changes are limited to this plan, the Tauri tray feature, tray.rs, window.rs, and lib.rs. Keep the result on feat/shell-lifecycle; do not merge or push main.

## Acceptance criteria

1. Tauri creates one tray icon using the existing application icon.
2. The tray menu contains exactly Show, Hide, and Quit.
3. Show and Hide affect only the main overlay and do not panic when it is unavailable.
4. Close and Alt+F4 hide the overlay without terminating the process or collector.
5. Quit requests normal Tauri exit so RunEvent::Exit shuts down the live collector.
6. Initial collection, live notifications, reconciliation, retry/backoff, summary events, and SQLite persistence remain unchanged.
7. No new frontend API, Tauri capability, network path, telemetry, sidecar, or raw provider data is introduced.
8. Automated gates, release build, and Windows 11 shell/live smoke verification pass.

## Explicitly deferred follow-up slices

- Settings window and validated root overrides.
- Launch on Windows login and single-instance enforcement.
- Remembered multi-monitor position, opacity, and configurable always-on-top.
- Explicit WSL UNC selection and source-root recovery UX.
- Clear-index confirmation, backup/rebuild, installer, and clean uninstall.
