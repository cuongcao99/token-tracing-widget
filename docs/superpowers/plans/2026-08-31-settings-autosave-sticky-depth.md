# Settings auto-save, sticky header, and surface-depth implementation plan

> **For agentic workers:** Keep tests ahead of production changes. Preserve
> the existing native Tauri bridge and the local-only product boundaries.

**Goal:** Make settings changes auto-save, keep the settings header visible
while only its content scrolls, stabilize dimensions around scrollbar changes,
and bring the widget title/elevation into visual balance with the settings
surface.

**References:** `CONTEXT.md`, `PRODUCT.md`,
`design/DESIGN_APPLE.md`, `design/DESIGN_CLAUDE.md`,
`docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`,
`docs/superpowers/specs/2026-08-31-widget-polish-design-update.md`, and
`docs/superpowers/specs/2026-08-31-settings-autosave-sticky-layout-design-update.md`.

## Constraints

- Windows 11, local-only, metadata-only; no network, telemetry, frontend state
  library, CSS framework, ORM, or bundled font.
- Keep settings preview immediate across windows and preserve typed Tauri
  bridges for persistence, preview, native dragging, and native resizing.
- Keep the Rust filesystem, collection, source, and SQLite ownership unchanged.
- Do not stage `.impeccable/`, dependencies, generated output, or local
  `.claude/` settings.

## Task 1: Add failing frontend contracts

**Files:**

- Modify: `src/tests/components/settings/SettingsScreen.test.tsx`
- Modify: `src/tests/lib/widget-layout.test.ts`

Replace explicit-save assertions with failing coverage that toggling provider
visibility, source collection, and dark mode calls their persistence bridge
without a Save button. Add source-root debounce and blur-flush coverage with
fake timers, verify close no longer restores the old snapshot, and assert the
settings root contains a separate scroll body. Update widget layout expectations
for the 32px title-safe targets. Run the focused tests and record the expected
failures before implementation.

## Task 2: Implement serialized auto-save

**Files:**

- Modify: `src/hooks/useSettingsController.ts`
- Modify: `src/components/settings/SettingsScreen.tsx`
- Modify: `src/components/settings/SourceSettingsSection.tsx`
- Modify: `src/components/settings/settings-model.ts` if a snapshot helper is
  needed

Replace `save`/`saved` state with a latest-snapshot persistence queue. Every
  state-changing toggle emits the complete preview and enqueues the matching
  source/widget persistence snapshot. Debounce source-root text edits for
  roughly 350ms, flush on blur, normalize roots before writing, and prevent
  stale queued writes from overtaking newer snapshots. Keep an inline error
  status and make close await pending work without preview rollback. Remove the
  Save button and submit-only actions while retaining accessible form controls.

## Task 3: Implement sticky settings content and stable dimensions

**Files:**

- Modify: `src/components/settings/SettingsScreen.tsx`
- Modify: `src/styles/settings.css`
- Modify: `src/styles/tokens.css`
- Modify: `src-tauri/src/app/tray.rs`
- Modify: `src-tauri/src/app/tray.rs` tests

Keep the panel root as a non-scrolling flex surface, move statuses and form
into a `settings-page__body` scroll region with `min-height: 0`, `overflow-y:
auto`, and `scrollbar-gutter: stable`, and keep the header outside that region.
Move scrollbar styling to the body. Raise the default settings height to 600px
and keep native bounds consistent.

## Task 4: Rebalance widget typography and elevation

**Files:**

- Modify: `src/styles/tokens.css`
- Modify: `src/styles/widget.css`
- Modify: `src/styles/settings.css`
- Modify: `src/lib/widget-layout.ts`
- Modify: `src/tests/lib/widget-layout.test.ts`

Set the widget title token to the settings title token. Increase responsive
target heights to preserve provider data and vertical breathing room. Replace
the current shallow surface shadows with layered CSS elevation for light and
dark surfaces, keeping `border: 0`, `outline: none`, and native shadow disabled.

## Task 5: Verify and update maintained context

**Files:**

- Modify: `CONTEXT.md`

Record auto-save semantics, sticky settings body ownership, stable scrollbar
gutter, 600px default height, and layered CSS elevation. Run focused tests,
full frontend tests, `npm run build`, Rust fmt/check/test, Tauri debug build,
`git diff --check`, and the Impeccable detector on changed UI files. Review
that no old save/restore or full-panel scrolling path remains.
