# Implementation Plan: Apple-Inspired Overlay Presentation Slice

**Date:** 2026-08-30
**Status:** Ready for implementation approval; no source code changed by this plan
**Design authority:** `docs/superpowers/specs/2026-08-30-overlay-apple-redesign-design-update.md` and user-provided `DESIGN.md`

## Goal

Redesign the existing Token Tracing overlay and Settings page into a restrained
Apple-inspired visual system, using the approved Dark Focus Tile comp for the
320 × 120 overlay, while changing the main window from always-on-top to a
normal non-topmost utility window. Preserve all existing collection, typed
summary, settings-command, tray, privacy, and lifecycle behavior.

## Architecture

Keep the existing architecture and file ownership:

- React/TypeScript owns presentation, semantic markup, formatting, and typed
  Tauri bridge usage.
- Plain CSS owns the visual tokens, layout, focus states, high-DPI-safe sizing,
  and reduced-motion behavior.
- Tauri configuration owns the main window's topmost/taskbar/decoration flags.
- Rust shell code continues to own close-to-hide and tray lifecycle behavior;
  no collection or SQLite module changes are expected.
- `UsageSummary`, source settings commands, event names, and payload validation
  remain unchanged.

## Tech Stack

- Tauri 2, Rust, React 19, TypeScript, Vite, Vitest, plain CSS
- Existing `@tauri-apps/api` event/core bridge
- Existing Rust test and integration suites
- No new dependency, CSS framework, frontend state library, font package,
  network client, sidecar, or ORM

## Spec

Implement only the approved design update:

- Main surface: dark `#272729` tile, white/muted text, blue-on-dark status,
  hairlines, no shadow/gradient, compact radius.
- Main hierarchy: provider/state header, dominant current-session number,
  Today/footer readout, relative update text.
- Visible metric values are numeric; accessible names include `tokens`.
- Header is draggable; body is not click-through and is not a drag region.
- Main window is normal non-topmost, frameless, transparent, non-resizable, and
  taskbar-hidden; close hides it.
- Settings uses the same token family on a light parchment functional surface,
  with no new settings features or command changes.
- Tray actions remain Show/Hide/Settings/Quit.

## Global Constraints

- Work on `dev`; do not merge or push to `main`.
- Do not modify or stage the user-provided `DESIGN.md` unless explicitly asked.
- Do not widen the privacy boundary: prompts, responses, reasoning, tool
  payloads, credentials, repository contents, working directories, raw
  records, and arbitrary file paths must not enter frontend payloads,
  diagnostics, SQLite, or tests.
- Do not change provider adapters, collection-core invariants, checkpoints,
  source discovery, live watcher behavior, or database schema.
- Keep the implementation scoped to presentation, window configuration, and
  narrow behavior/accessibility tests.
- Use the existing `.impeccable/build/spec.json` for proportions. It was
  measured from a 2048 × 768 generated reference raster; normalize those
  measurements to the real 320 × 120 logical-pixel Tauri viewport instead of
  copying raster pixels directly as CSS font sizes.

## Implementation tasks

### 1. Add regression tests for the presentation contract first

Files:

- `src/App.test.tsx`
- `src/Settings.test.tsx` only where markup roles or accessible names change

Steps:

- [ ] Replace string-fragment assertions that require `"1,234 tokens"` or
      `"Today: 5,678 tokens"` with assertions for the visible numeric values
      and their semantic accessible names containing the unit.
- [ ] Add a test that active summaries render provider, state, current-session
      number, Today number, and relative update text in the new semantic
      structure.
- [ ] Add a test for an undefined current-session value that renders the
      unavailable value without changing the Today total or overflowing the
      semantic label.
- [ ] Add a test for a large Today total and a long provider name to prove the
      markup remains bounded and does not reintroduce raw content.
- [ ] Preserve and rerun the existing drag-region assertion on the header.
- [ ] Preserve the event-listener cleanup and invalid-summary behavior tests;
      the redesign must not alter the bridge lifecycle.
- [ ] If Settings roles change, update only the affected tests and continue to
      cover both provider settings, save success, and sanitized save errors.

Verification command:

```powershell
npm test -- --run
```

### 2. Refactor the overlay markup without changing its data contract

File:

- `src/App.tsx`

Steps:

- [ ] Keep `UsageSummary`, `getUsageSummary`,
      `listenForUsageSummary`, `formatRelativeUpdate`, and all error fallback
      behavior unchanged.
- [ ] Keep the existing provider fallback (`Token Tracing`) and state-label
      mapping, but render the state as a semantic status with an accessible
      text signal independent of color.
- [ ] Split the current concatenated metric strings into labeled numeric
      outputs so the visual comp can omit repeated units while `aria-label`
      values remain explicit, e.g. `Current session: 1,234 tokens`.
- [ ] Preserve `data-tauri-drag-region=""` on the header and ensure only the
      header carries the drag-region marker.
- [ ] Use a stable heading and landmark structure for the compact overlay;
      keep the `aria-label="Token usage summary"` purpose clear.
- [ ] Avoid adding buttons, icons, charts, hidden raw payloads, or a new state
      store.

### 3. Implement the Dark Focus Tile design system in plain CSS

File:

- `src/styles.css`

Steps:

- [ ] Define local CSS custom properties for the approved Apple-inspired
      palette, spacing, radius, type roles, focus ring, and hairline; keep raw
      values centralized rather than repeated in selectors.
- [ ] Replace the light gray overlay with the dark `#272729` surface, white
      primary text, muted secondary text, blue-on-dark status, hairlines, and
      no shadow or gradient.
- [ ] Build a fixed logical layout for 320 × 120 using explicit grid/flex
      tracks that keep the header, hero, and footer readable at Windows 100%,
      125%, and 150% scaling. Remove the existing resolution media query that
      shrinks text and padding below readable logical sizes.
- [ ] Give the hero number the strongest typographic scale and tight display
      tracking; keep labels and update metadata quiet but readable.
- [ ] Add bounded text behavior for provider names, large totals, and
      unavailable states; verify no horizontal scrollbar or clipped footer.
- [ ] Keep cursor semantics for the draggable header (`grab`/`grabbing`) and
      ensure non-header content does not imply dragging.
- [ ] Add `:focus-visible` styles for Settings controls using Focus Blue and
      no focus treatment that relies on color alone.
- [ ] Add a `prefers-reduced-motion: reduce` rule even if the default design
      uses no decorative animation.
- [ ] Restyle the Settings page in the same token family on a light parchment
      canvas: remove existing card shadows, use hairlines/functional spacing,
      preserve readable inputs/toggles/buttons, and prevent narrow-window
      overflow.
- [ ] Keep CSS plain and local; do not introduce a CSS framework or icon font.

### 4. Apply the explicit window behavior update

File:

- `src-tauri/tauri.conf.json`

Steps:

- [ ] Change only the main window's `alwaysOnTop` default from `true` to
      `false`.
- [ ] Preserve width 320, height 120, non-resizable, frameless, transparent,
      and skip-taskbar settings.
- [ ] Do not add an Always on top setting until its separate persistence and
      command design is approved.

Files to verify remain unchanged unless a focused smoke-test failure proves
otherwise:

- `src-tauri/src/app/window.rs`
- `src-tauri/src/app/tray.rs`
- `src-tauri/capabilities/default.json`

Expected behavior to verify:

- [ ] Close/Alt+F4 still hides the main window through the existing
      `CloseRequested` handler.
- [ ] Tray Show/Hide/Settings/Quit still map to the same IDs and actions.
- [ ] Showing or updating the overlay does not steal focus from another
      foreground window.

### 5. Run the visual build-phase checks after implementation exists

Files/artifacts:

- `.impeccable/mocks/overlay-dark-focus-tile.png`
- `.impeccable/mocks/overlay-dark-focus-tile.png.json`
- `.impeccable/build/spec.json`
- `.impeccable/build/scaffold/layout.css`
- `.impeccable/review/hero-repro.png`

Steps:

- [ ] Run `node C:\Users\caocu\.codex\plugins\cache\impeccable\impeccable\4.1.2\skills\impeccable\scripts\build-phase.mjs scaffold`
      from the repository root and inspect the generated measured layout
      before binding CSS.
- [ ] Bind the measured proportions to the semantic elements in `App.tsx`;
      keep the 320 × 120 logical viewport and do not copy the 2048 × 768
      raster's absolute dimensions.
- [ ] Capture the first-viewport reproduction at
      `.impeccable/review/hero-repro.png`, then run
      `node C:\Users\caocu\.codex\plugins\cache\impeccable\impeccable\4.1.2\skills\impeccable\scripts\build-phase.mjs record hero --build .impeccable\review\hero-repro.png`.
- [ ] After the hero result is accepted, run
      `node C:\Users\caocu\.codex\plugins\cache\impeccable\impeccable\4.1.2\skills\impeccable\scripts\build-phase.mjs advance`.
- [ ] Run the detector once against the changed UI targets with
      `node C:\Users\caocu\.codex\plugins\cache\impeccable\impeccable\4.1.2\skills\impeccable\scripts\detect.mjs src\App.tsx src\Settings.tsx src\styles.css`;
      address actionable findings without widening scope.
- [ ] Review the actual overlay at normal and high-DPI Windows scaling for
      baseline alignment, contrast, clipping, and drag affordance.

### 6. Verify frontend, Rust shell, integration, and privacy boundaries

Steps:

- [ ] Run `npm test -- --run`.
- [ ] Run `npm run build` and confirm both `index.html` and `settings.html`
      still emit.
- [ ] Run `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`.
- [ ] Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `npm run tauri build -- --debug`.
- [ ] Confirm no new frontend payload field, raw provider value, filesystem
      read, network permission, or dependency was introduced.
- [ ] If any cross-boundary change becomes necessary, stop and update the
      design/spec before editing Rust interfaces.

### 7. Perform the Windows 11 smoke pass

Use the debug executable produced by the integrated build.

- [ ] Start the app and confirm the overlay is frameless, transparent,
      taskbar-hidden, and initially 320 × 120.
- [ ] Put Notepad/Explorer in front and confirm it covers the overlay; confirm
      the overlay is not click-through when brought forward.
- [ ] Drag using the header only; confirm the body/footer do not unexpectedly
      drag the window.
- [ ] Close the overlay with the normal close gesture/Alt+F4; confirm it hides
      and the collector remains alive.
- [ ] Use tray Show, Hide, Settings, and Quit; confirm Settings is single,
      decorated, resizable, non-topmost, and focused when opened.
- [ ] Toggle and save Claude Code/Codex source settings; confirm the visual
      redesign did not change persistence, validation, watcher refresh, or
      privacy-safe messaging.
- [ ] Exercise active, idle, unavailable, stale, loading, missing-session, and
      large-total states; confirm each is legible and bounded.
- [ ] Repeat at 125% and 150% display scaling and on a second monitor if
      available; confirm no clipping and no position regression.

## Completion gate

Do not claim the slice complete until the design update is accepted, the
frontend/Rust/integrated commands pass, the impeccable visual gate has a
recorded hero result, and the Windows smoke checklist passes. Until then,
remain on `dev` and leave `main` untouched.
