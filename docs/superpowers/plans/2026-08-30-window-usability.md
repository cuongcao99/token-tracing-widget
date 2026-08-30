# Window Usability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the existing 320x120 frameless overlay usable by removing document overflow and enabling native dragging from its non-interactive header.

**Architecture:** Keep window behavior at the existing Tauri/React boundary. React declares the header as a native Tauri drag region, the existing Tauri capability grants only the native drag command required by that attribute, and plain CSS makes the HTML viewport, root, and widget share the fixed Tauri window dimensions while safely shrinking/wrapping metric children. No Rust command, watcher, tray, settings flow, or data-boundary change is needed.

**Tech Stack:** Tauri 2, React 19, TypeScript, Vite, Vitest, Testing Library, and plain CSS.

**Spec:** `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`

## Global Constraints

- Version 1 remains local-only and Windows 11-only.
- Use Tauri 2, Rust, React, TypeScript, Vite, plain CSS, and SQLite as already approved; do not add a frontend state library, CSS framework, ORM, background service, sidecar, network client, or telemetry.
- The overlay is transparent, frameless, always-on-top by default, and absent from the taskbar; this slice changes only the existing drag usability.
- The overlay is approximately 320 by 120 logical pixels and can be dragged.
- Rust owns filesystem and SQLite access; the React webview receives typed summaries only.
- Prompts, responses, reasoning, tool payloads, credentials, repository contents, and working directories never enter normalized events, SQLite, diagnostics, or frontend payloads.
- Prefer the native Tauri drag-region mechanism over adding a Rust command or dependency.
- Do not implement filesystem watching, the 30-second reconciliation pass, tray actions, settings, startup registration, single-instance behavior, remembered position, opacity controls, WSL discovery, network access, or installer/uninstaller behavior in this slice.

---

## Scope and baseline

The handoff identifies the current checkout as `codex/window-usability`, clean at `1f227a6`, with `main` and `dev` also at that commit. The prior runtime plan is `docs/superpowers/plans/2026-08-30-runtime-integration.md`.

The current UI has no interactive controls. `src/App.tsx` renders a header, two metric blocks, and a relative-update footer. `src/styles.css` gives `body` document-level minimum dimensions, gives the widget only a `min-height`, and does not constrain the root/document overflow. The metrics are flex children without explicit `min-width: 0`, so a large daily value can widen the page and produce scrollbars. `src-tauri/tauri.conf.json` already supplies the fixed 320x120, frameless, transparent, always-on-top window and needs no change.

The visible frontend regression tests are in `src/App.test.tsx`; they already cover summary rendering, event updates/listener cleanup, and unavailable fallback. Keep those tests. Add the smallest drag-region contract test; retain the existing large-value coverage if present in the execution checkout and add the concrete large-value rendering check below because this checkout currently has no named large-value test.

## File map

- Modify `src/App.tsx` to mark only the current non-interactive header with `data-tauri-drag-region=""`.
- Modify `src/App.test.tsx` to prove the drag-region contract and preserve summary/unavailable behavior; add one large-number rendering regression if it is still absent.
- Modify `src/styles.css` to fill the Tauri viewport, remove document minimum dimensions, constrain overflow, make metric children shrink/wrap, and show grab/grabbing cursors.
- Modify `src-tauri/capabilities/default.json` to grant only `core:window:allow-start-dragging`, which the native drag-region implementation invokes.
- Review `src-tauri/tauri.conf.json` to confirm the existing 320x120 frameless window is sufficient; do not modify it.
- Review `src-tauri/src/app/window.rs` to confirm the scaffold remains untouched; do not implement the later shell window slice here.

## Interfaces

- The DOM contract produced by `App` is a header carrying the empty `data-tauri-drag-region` attribute. Tauri 2 recognizes the attribute and invokes its native drag command through the narrowly scoped `core:window:allow-start-dragging` capability permission; no Rust command is added.
- `UsageSummary`, `getUsageSummary`, `listenForUsageSummary`, the `usage-summary-changed` event, and the Rust privacy boundary remain unchanged.
- The Tauri window remains the existing `main` window at 320x120 logical pixels.

---

### Task 1: Add the native drag-region contract

**Files:**
- Modify: `src/App.tsx:82-88`
- Test: `src/App.test.tsx`
- Modify: `src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: the existing non-interactive `widget__header` rendered by `App`.
- Produces: a header with `data-tauri-drag-region=""`; no new Tauri command, event, or dependency. The existing capability receives only the permission required for Tauri's native drag-region command.

- [x] **Step 1: Write the failing UI regression test.**

Add this test to `src/App.test.tsx` without removing the existing summary and unavailable tests:

```tsx
it("marks the non-interactive header as a Tauri drag region", () => {
  render(<App />);

  expect(screen.getByRole("banner")).toHaveAttribute(
    "data-tauri-drag-region",
    "",
  );
});
```

The test must assert the attribute on the header, not on the whole `main`, so future controls can remain outside the drag surface without changing this contract.

- [x] **Step 2: Run the focused test and verify it fails for the missing attribute.**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: the existing tests pass or reach their current baseline, and the new test fails because the banner has no `data-tauri-drag-region` attribute. If this environment reports the known esbuild `spawn EPERM`, rerun this exact command in the approved elevated execution context; do not alter application code to work around it.

- [x] **Step 3: Add the Tauri drag attribute to the header.**

Change only the existing header opening tag in `src/App.tsx`:

```tsx
<header
  className="widget__header"
  data-tauri-drag-region=""
>
```

Keep the header contents, summary state rendering, and all Rust/React data flow unchanged. Use an empty string rather than a boolean JSX value so React emits a present `data-*` attribute reliably. Do not import `getCurrentWindow`, call `startDragging()`, add an `onMouseDown` handler, or add a Rust command.

- [x] **Step 4: Grant the native drag command in the existing capability.**

Update `src-tauri/capabilities/default.json` without changing the window configuration or adding any other permission:

```json
"permissions": [
  "core:default",
  "core:window:allow-start-dragging"
]
```

Tauri's drag-region script delegates to `plugin:window|start_dragging`; `core:default` does not include that command, so the explicit permission is required for the DOM contract to work in the packaged app. Keep all other capability entries unchanged.

- [x] **Step 5: Run the focused frontend tests and verify the drag contract passes.**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: the drag-region test and all existing `App` tests pass, including event-listener cleanup and unavailable fallback.

- [x] **Step 6: Commit the isolated DOM contract and capability permission.**

```powershell
git add src/App.tsx src/App.test.tsx src-tauri/capabilities/default.json
git commit -m "feat: make overlay header draggable"
```

### Task 2: Contain the fixed overlay layout

**Files:**
- Modify: `src/styles.css`
- Test: `src/App.test.tsx` only if the large-number regression is absent

**Interfaces:**
- Consumes: the existing `widget`, `widget__header`, `widget__metrics`, metric child, and `widget__note` class names.
- Produces: a 100%-sized widget inside a 100%-sized hidden-overflow document, with shrinkable metric columns and a visible drag cursor; no changed summary payload or text contract.

- [x] **Step 1: Preserve or add the large-number UI regression.**

Do not delete the existing large-number test if the execution checkout contains one. If it is absent, add this test to `src/App.test.tsx` so the layout change cannot silently remove or alter the displayed value:

```tsx
it("renders a large daily total without changing the metric text", async () => {
  vi.mocked(getUsageSummary).mockResolvedValueOnce({
    state: "active",
    provider: "Codex",
    currentSessionTokens: 1234,
    todayTokens: 13650796,
    lastUpdatedAt: "2026-08-30T00:00:01Z",
    sourceHealth: [{ provider: "Codex", state: "active" }],
  });

  render(<App />);

  expect(
    await screen.findByText("Today: 13,650,796 tokens"),
  ).toBeInTheDocument();
});
```

This is a content-preservation check; jsdom cannot provide trustworthy native WebView layout metrics. The actual no-scrollbar and readability proof is the Windows release smoke check in Task 3.

- [x] **Step 2: Replace document minimum sizing with viewport sizing and real overflow constraints.**

In `src/styles.css`, keep the existing global `box-sizing: border-box` rule and make the document chain fill the Tauri viewport:

```css
html,
body,
#root {
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

body {
  margin: 0;
  background: transparent;
}
```

Remove `body`'s `min-width: 320px` and `min-height: 120px`; those dimensions belong to the Tauri window, not to a scrollable document.

- [x] **Step 3: Make the widget fill, rather than exceed, the fixed window.**

Update `.widget` as follows, retaining its existing background, border, and radius:

```css
.widget {
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  padding: 12px;
  margin: 0;
  overflow: hidden;
  background: #f5f6f8;
  border: 1px solid #d8dde3;
  border-radius: 10px;
}
```

The reduced padding and existing fixed 320x120 window keep the header, metrics, and footer inside the available content box. Do not change `tauri.conf.json` dimensions or make the window resizable.

- [x] **Step 4: Give flex/grid children an explicit shrink and wrap strategy.**

Keep the current 12px horizontal gap, but use bounded grid tracks and compact vertical spacing:

```css
.widget__header,
.widget__metrics {
  min-width: 0;
}

.widget__header h1,
.widget__metrics > div,
.widget__metrics strong {
  min-width: 0;
}

.widget__header h1 {
  overflow-wrap: anywhere;
}

.widget__metrics {
  display: grid;
  grid-template-columns: minmax(0, 0.8fr) minmax(0, 1.2fr);
  align-items: end;
  gap: 12px;
  margin-top: 12px;
}

.widget__metrics > div {
  display: grid;
  gap: 4px;
}

.widget__metrics strong {
  display: block;
  overflow-wrap: anywhere;
}

.widget__note {
  margin: 8px 0 0;
  min-width: 0;
  overflow-wrap: anywhere;
  color: #6b7280;
  font-size: 0.7rem;
}
```

The `minmax(0, ...)` tracks and `min-width: 0` declarations prevent intrinsic token text from widening the page. `overflow-wrap: anywhere` keeps a long formatted count readable inside its metric column instead of creating horizontal overflow. Do not use hidden-scrollbar styling, `white-space: nowrap`, or an ellipsis that hides token digits as the fix.

- [x] **Step 5: Add the grab cursor to the same non-interactive drag surface.**

Add these declarations to `src/styles.css`:

```css
.widget__header {
  cursor: grab;
}

.widget__header:active {
  cursor: grabbing;
}
```

Keep the drag surface limited to the header. Do not make the entire widget draggable, because later settings or tray-related controls must not inherit a drag region accidentally.

At the current Windows 11 168% display scale, add a narrow `@media (min-resolution: 1.5dppx)` compaction rule for the fixed logical viewport: reduce widget padding to `4px`, reduce the header/status/labels/footer and metric-value font sizes, and tighten metric/footer spacing. This keeps the same content readable in the physical WebView without changing the logical Tauri window dimensions or summary text.

- [x] **Step 6: Run the frontend regression suite and build.**

Run:

```powershell
npm test -- --run
npm run build
```

Expected: all frontend tests pass, including the existing summary/event/unavailable checks and the large-number rendering check, and TypeScript/Vite build successfully without a new dependency.

- [x] **Step 7: Commit the layout fix.**

```powershell
git add src/styles.css src/App.test.tsx
git commit -m "fix: contain overlay layout overflow"
```

### Task 3: Verify the release artifact and scope boundary

**Files:**
- Review: `src/App.tsx`
- Review: `src/App.test.tsx`
- Review: `src/styles.css`
- Review: `src-tauri/capabilities/default.json`
- Verify unchanged: `src-tauri/tauri.conf.json`
- Verify unchanged: `src-tauri/src/app/window.rs`
- Verify artifact: `src-tauri/target/release/token-tracing-widget.exe`

**Interfaces:**
- The shipped executable remains one Tauri process with the existing Rust runtime and one-shot startup collection.
- The only new UI contract is the header's native `data-tauri-drag-region` attribute, backed by the single `core:window:allow-start-dragging` capability permission; the `UsageSummary` allow-list and Rust-to-React privacy boundary are unchanged.

- [x] **Step 1: Run the Rust checks even though no Rust file should change.**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
cargo check --manifest-path src-tauri/Cargo.toml --offline
```

Expected: the existing Rust suite passes with no Rust source change from this slice. The capability JSON is validated by the integrated Tauri build. No watcher, tray, startup, or window Rust scaffold is implemented.

- [x] **Step 2: Build the integrated release executable.**

Run:

```powershell
npm run tauri build -- --no-bundle
```

Expected: `src-tauri/target/release/token-tracing-widget.exe` is rebuilt from the updated frontend and remains a single no-bundle Tauri executable with the existing Windows GUI subsystem behavior.

- [x] **Step 3: Perform the Windows release smoke check.**

Launch `src-tauri/target/release/token-tracing-widget.exe` from Explorer and verify all of the following:

- The overlay remains approximately 320x120, transparent, frameless, always-on-top, and absent from the taskbar.
- No horizontal or vertical scrollbar is visible.
- Dragging the header moves the overlay; the cursor changes from grab to grabbing while pressed.
- A large daily value such as `13,650,796` remains readable and does not widen the page or clip its digits.
- Active, idle, loading, and unavailable content remains rendered as before; the existing collection result and relative-update footer are unchanged.
- No manual refresh, watcher, polling timer, tray menu, settings UI, position persistence, WSL discovery, or new network behavior appears as a side effect.
- No prompt, response, reasoning, tool payload, credential, repository content, working directory, raw JSON, or absolute source path is displayed.

Stop the executable after the check.

- [x] **Step 4: Check the final diff is limited to the approved slice.**

Run:

```powershell
git diff --check
git status --short --branch
git diff --name-only 1f227a6..HEAD
git diff -- src-tauri/src
git diff -- src-tauri/capabilities/default.json
```

Expected: tracked implementation changes are limited to `src/App.tsx`, `src/App.test.tsx`, `src/styles.css`, and the single `core:window:allow-start-dragging` permission in `src-tauri/capabilities/default.json`; `git diff -- src-tauri/src` is empty. Do not stage generated `dist`, `src-tauri/target`, `.claude/`, or provider data.

## Acceptance criteria for this slice

1. The release overlay has no horizontal or vertical scrollbar.
2. The existing 320x120 Tauri window remains non-resizable, transparent, frameless, always-on-top, and absent from the taskbar.
3. Dragging the non-interactive header moves the window through Tauri's native drag-region mechanism.
4. Large formatted token values stay inside the widget layout and remain readable without changing the `UsageSummary` contract.
5. Existing active, idle, loading, unavailable, event-update, cleanup, and privacy-boundary behavior remains green.
6. No Rust source, dependency, watcher, tray, settings, startup, or collection behavior changes in this slice; the only Tauri capability change is `core:window:allow-start-dragging` required by the native drag region.
7. Frontend tests, frontend build, Rust checks, integrated Tauri build, and Windows smoke verification pass.

## Explicitly deferred follow-up

After this slice is green, the next product slice is the live collection loop: filesystem notifications plus the bounded 30-second reconciliation pass, with summary events emitted only after successful SQLite commits. Tray/Settings/position persistence and explicit source-root configuration remain later shell/settings slices described by the approved spec and runtime integration plan.

## Plan self-review

### Spec coverage

- The presentation requirements for an approximately 320x120 draggable overlay are implemented by the viewport/widget sizing, native header drag region, and Windows smoke check.
- The approved React/Tauri typed summary seam is preserved; no raw observations or arbitrary files are introduced.
- The fixed frameless/transparent/always-on-top/taskbar configuration is reviewed and left unchanged.
- The collection data flow, one-shot startup behavior, SQLite boundary, and provider adapters are explicitly outside the changed files and verified unchanged.
- The spec's later watcher, reconciliation, tray, settings, startup, positioning, WSL, and installer behavior is explicitly deferred.

### Placeholder scan

This plan contains concrete file paths, test code, CSS declarations, commands, expected outcomes, commit messages, and manual checks. It contains no unspecified implementation step or placeholder task.

### Type and boundary consistency

No TypeScript interface or Rust type changes. The only new frontend-to-shell contract is the presence of `data-tauri-drag-region=""` on the existing header, backed by one narrowly scoped Tauri capability permission. `UsageSummary` remains the sole data payload used by the UI, and all privacy constraints remain those already enforced by the runtime slice.
