# Responsive Resizable Windows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or **superpowers:executing-plans** to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Claude-editorial widget responsive to provider visibility, allow native border resizing for both frameless windows, expose a six-dot drag affordance, and restore a balanced widget type scale.

**Architecture:** Keep sizing and resize direction decisions in focused TypeScript modules. The widget derives a target height from the number of visible providers and asks the current Tauri window to change only its height while preserving a user-resized width. Edge handles call Tauri's native `startResizeDragging` API; the header and six-dot grip continue to use the existing drag bridge. Rust/Tauri window configuration supplies the resizable flag and safe logical bounds for both windows.

**Tech Stack:** React 19, TypeScript, Vitest, Testing Library, plain CSS, Tauri 2 window API, Rust Tauri window builders, existing Claude-editorial tokens.

**Spec:** `docs/superpowers/specs/2026-08-30-claude-editorial-multi-provider-design-update.md`, preserving `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md` and `DESIGN_CLAUDE.md`.

## Global Constraints

- Keep version one local-only and Windows 11-only.
- Keep filesystem, collection, SQLite, and provider parsing in Rust; this slice changes window presentation only.
- Do not add a frontend state library, CSS framework, network client, telemetry, sidecar, or persistence for window geometry.
- Preserve the approved Claude editorial visual system: warm cream/coral/dark surfaces, serif display roles, restrained borders, and focused component folders.
- Use native Tauri resize/drag APIs rather than implementing a JavaScript window-movement or polling loop.
- Provider visibility remains the only input to automatic widget height; source health and token values must not affect geometry.
- Do not modify `DESIGN.md`, `DESIGN_CLAUDE.md`, or `.claude/` settings.

---

### Task 1: Define the responsive widget-height contract with tests first

**Files:**
- Create: `src/lib/widget-layout.ts`
- Create: `src/tests/lib/widget-layout.test.ts`

**Interfaces:**
- `WIDGET_DEFAULT_WIDTH`, `WIDGET_MIN_WIDTH`, `WIDGET_MAX_WIDTH`, `WIDGET_MIN_HEIGHT`, `WIDGET_MAX_HEIGHT`.
- `widgetHeightForVisibleProviders(visibleProviderCount: number): number`.

- [x] **Step 1: Write failing tests for zero, one, and two visible providers.**

Assert the approved default height is used for two providers, one provider leaves a compact but breathable panel, zero providers remains a valid minimum, and out-of-range counts clamp rather than produce an invalid height.

Run: `npm test -- --run src/tests/lib/widget-layout.test.ts`

Expected: FAIL because the layout module does not exist.

- [x] **Step 2: Implement the pure height mapping and bounds.**

Use explicit logical-pixel constants: 440px default width, 360–720px width bounds, 176px minimum height, 520px maximum height, and distinct 176/228/300px targets for 0/1/2 visible Providers. Keep the mapping independent of provider names, health, and token data.

- [x] **Step 3: Run the focused layout tests.**

Run: `npm test -- --run src/tests/lib/widget-layout.test.ts`

Expected: PASS.

---

### Task 2: Add typed window resize actions and focused component tests

**Files:**
- Modify: `src/lib/window-actions.ts`
- Create: `src/components/shared/WindowGrip.tsx`
- Create: `src/components/shared/WindowResizeHandles.tsx`
- Create: `src/tests/components/shared/WindowControls.test.tsx`

**Interfaces:**
- `startCurrentWindowResize(direction: ResizeDirection): Promise<void>`.
- `WindowGrip` renders six decorative dots without Unicode icon glyphs.
- `WindowResizeHandles` renders eight transparent edge/corner handles and invokes the native direction on primary-button press.

- [x] **Step 1: Write failing tests for the six-dot grip and all resize directions.**

Mock `@tauri-apps/api/window`, assert the grip has six dot elements and is hidden from assistive technology, then fire `mouseDown` on representative edge/corner handles and assert `startResizeDragging` receives the exact Tauri direction. Assert resize handles prevent the event from becoming a header drag.

Run: `npm test -- --run src/tests/components/shared/WindowControls.test.tsx`

Expected: FAIL because the components and resize bridge do not exist.

- [x] **Step 2: Implement the bridge and controls.**

Reuse `getCurrentWindow()`. Call `startResizeDragging` only after preventing default and stopping propagation. Keep handles transparent but focusable with the existing global focus-visible treatment; use CSS cursors and an eight-direction map. Keep the grip decorative and pointer-transparent so its parent header remains draggable.

- [x] **Step 3: Run the focused control tests.**

Run: `npm test -- --run src/tests/components/shared/WindowControls.test.tsx`

Expected: PASS.

---

### Task 3: Enable bounded native resizing in the Tauri shell

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/app/tray.rs`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/app/tray.rs` tests

**Interfaces:**
- Main overlay: resizable, 360–720px wide, 176–520px high.
- Settings window: resizable, 440–820px wide, 420–900px high.

- [x] **Step 1: Extend the existing shell configuration assertions.**

Update the tray option test and add JSON/config assertions for resizable state and logical min/max bounds before changing the shell. The assertions must prove the windows remain frameless, transparent, shadowed, taskbar-free, and non-always-on-top as already approved.

Run: `cargo test --manifest-path src-tauri/Cargo.toml app::tray`; run the existing configuration assertion if present.

Expected: FAIL because the current windows are explicitly non-resizable and have no bounds.

- [x] **Step 2: Implement the native configuration.**

Set `resizable: true` and add the main-window bounds in `tauri.conf.json`. Add settings bounds to `SettingsWindowOptions` and apply `min_inner_size`/`max_inner_size` on `WebviewWindowBuilder`. Add only the specific core window permissions required by `startResizeDragging`, `setSize`, `innerSize`, and `scaleFactor`.

- [x] **Step 3: Run Rust and config checks.**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`; `cargo test --manifest-path src-tauri/Cargo.toml app::tray`; and the existing Tauri JSON assertion.

Expected: PASS.

---

### Task 4: Make the widget auto-size from visible Providers and add grips/handles

**Files:**
- Create: `src/lib/window-sizing.ts`
- Modify: `src/components/widget/WidgetHeader.tsx`
- Modify: `src/components/widget/TokenTracingWidget.tsx`
- Modify: `src/components/settings/SettingsScreen.tsx`
- Modify: `src/styles/index.css`
- Create: `src/styles/window-controls.css`
- Modify: `src/styles/widget.css`
- Modify: `src/styles/settings.css`
- Modify: `src/tests/components/widget/TokenTracingWidget.test.tsx`
- Modify: `src/tests/components/settings/SettingsScreen.test.tsx`

**Interfaces:**
- `syncWidgetWindowHeight(visibleProviderCount: number): Promise<void>` preserves the current logical width and sets only the bounded target height.
- Widget visibility changes trigger the sync; source enabled previews do not.
- Both headers visibly contain the six-dot grip; both surfaces contain native resize handles.

- [x] **Step 1: Add failing widget/settings behavior assertions.**

Mock the sizing bridge and assert the widget requests height targets for two and one visible Providers. Assert a source toggle does not change the visible-provider count. Assert both Settings and widget render six-dot grips and resize handles, and Settings resize handles do not invoke the header drag action.

Run: `npm test -- --run src/tests/components/widget/TokenTracingWidget.test.tsx src/tests/components/settings/SettingsScreen.test.tsx src/tests/components/shared/WindowControls.test.tsx`

Expected: FAIL because no sizing effect or controls are mounted.

- [x] **Step 2: Implement the sizing bridge.**

Read `innerSize()` and `scaleFactor()`, convert the current physical width to logical pixels, clamp it to the approved width bounds, and call `setSize(new LogicalSize(currentWidth, targetHeight))`. Treat a native sizing failure as best-effort UI transport failure so a widget render remains usable. Serialize or supersede rapid calls so the latest visibility state wins.

- [x] **Step 3: Mount the controls without changing the approved structure.**

Add `WindowGrip` to each draggable header and `WindowResizeHandles` as an absolutely positioned child of each surface. Keep close/toggle/source buttons excluded from drag initiation. Keep the Settings content scrollable and place handles on the surface edge so they remain available while content scrolls.

- [x] **Step 4: Run focused frontend tests.**

Run the command from Step 1.

Expected: PASS.

---

### Task 5: Rebalance widget type scale and update the design contract

**Files:**
- Modify: `src/styles/tokens.css`
- Modify: `src/design-preview.css`
- Modify: `docs/superpowers/specs/2026-08-30-claude-editorial-multi-provider-design-update.md`

**Interfaces:**
- Widget title becomes legible without competing with Provider rows.
- Total number becomes secondary to the Provider values while remaining scannable.

- [x] **Step 1: Add a regression assertion for the semantic type tokens.**

Keep this as a CSS/source-level check if no CSS runtime is available: assert the shipped token declarations and design-preview overrides contain the same title/total values. Do not infer visual completion from a screenshot alone.

- [x] **Step 2: Implement the modest type adjustment.**

Set `--type-widget-title` to 19px and `--type-widget-total` to 20px, preserving the existing Copernicus/display and StyreneB/UI font roles, tabular numerals, and restrained weights. Mirror those values in the static design preview.

- [x] **Step 3: Record the approved responsive/resizable update.**

Append a dated design update documenting automatic 0/1/2-Provider height targets, bounded manual resizing, the six-dot drag affordance, and the title/Total hierarchy. No privacy or collection boundary changes are introduced.

---

### Task 6: Verify the full slice and inspect the result

**Files:**
- No new production files; inspect the complete diff.

- [x] **Step 1: Run frontend verification.**

Run: `npm test -- --run`; `npm run build`; `git diff --check`.

- [x] **Step 2: Run Rust and integrated verification.**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`; `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo test --manifest-path src-tauri/Cargo.toml`; `npm run tauri build -- --debug`.

- [ ] **Step 3: Review the final diff and smoke-test the packaged Windows app.**

Automated diff/build review is complete. Manual packaged-app smoke remains: confirm that hiding a Provider reduces the widget height, showing it restores the two-row target, both surfaces can be dragged by their header and resized from the edge/corner, and no visible white border is introduced. Confirm settings source/dark-mode preview/save behavior remains unchanged.
