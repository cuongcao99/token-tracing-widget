# Widget polish and day-scoped session implementation plan

> **For agentic workers:** Follow this plan task by task. Keep tests ahead of
> production changes and verify each boundary before moving on.

**Goal:** Fix the transparent perimeter artifact, make the six-dot grip the
actual top-center drag affordance, prevent widget resize from clipping provider
data, restore breathing room in provider rows, and scope current-session values
to the current Windows local day.

**Architecture:** Keep native window movement and resizing behind the existing
typed Tauri bridge. The shared grip owns movement initiation; the existing
eight native resize handles own border resizing. The widget sizing bridge keeps
the current logical width, updates the native minimum height from visible
provider count, and preserves the existing automatic 0/1/2-provider targets.
Keep historical events available for state and last-update metadata while a
small pure usage helper derives current-session tokens from current-day events.

**References:** `CONTEXT.md`, `PRODUCT.md`,
`design/DESIGN_APPLE.md`, `design/DESIGN_CLAUDE.md`,
`docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`,
`docs/superpowers/specs/2026-08-30-claude-editorial-multi-provider-design-update.md`,
and `docs/superpowers/specs/2026-08-31-widget-polish-design-update.md`.

## Constraints

- Windows 11, local-only, metadata-only; do not add network, telemetry,
  frontend state libraries, CSS frameworks, ORM, or font packages.
- Preserve typed settings preview/save/restore semantics and independent
  provider/source state.
- Keep Rust responsible for filesystem, collection, normalization, and SQLite;
  keep window movement/resizing native rather than polling from JavaScript.
- Do not stage `.impeccable/`, dependencies, generated build output, or local
  `.claude/` settings.

### Task 1: Lock the current-day session contract with failing Rust tests

**Files:**

- Modify: `src-tauri/tests/provider_summary.rs`
- Modify: `src-tauri/tests/collection_core.rs`

Add regression coverage for a historical Claude event from the previous local
day. The provider summary must preserve `Idle` and `last_updated_at`, return
`today_tokens == 0`, and expose `current_session_tokens == Some(0)`. Cover the
aggregate summary as well so the top-level bridge cannot retain the old session
value. Run the focused tests and confirm they fail against the current
all-history session calculation.

### Task 2: Implement a pure day-scoped session calculation

**Files:**

- Modify: `src-tauri/src/usage/active_provider.rs`
- Modify: `src-tauri/src/usage/provider_summary.rs`
- Modify: `src-tauri/src/collection/mod.rs`

Add a focused helper that filters events by the injected Windows local day,
uses the existing timestamp/session ordering for those events, and returns zero
when a provider has historical events but none in the current day. Continue to
use all valid historical provider events for state and last-update metadata.
Apply the helper to both provider summaries and the aggregate current-session
field without changing today's-total filtering or privacy fields. Run the
focused Rust tests, then format the changed Rust files.

### Task 3: Add failing frontend coverage for the real grip and safe sizing

**Files:**

- Modify: `src/tests/components/shared/WindowControls.test.tsx`
- Modify: `src/tests/components/widget/TokenTracingWidget.test.tsx`
- Modify: `src/tests/components/settings/SettingsScreen.test.tsx`
- Modify: `src/tests/lib/window-sizing.test.ts`

Replace the old decorative/header-drag assertions with tests that the grip is a
focusable button, starts the native drag bridge from its own mouse-down, and is
rendered once at the panel level for both surfaces. Assert title/header mouse
down does not start a move and resize handles still stop propagation. Extend
sizing tests to require `setSizeConstraints` with the target height before the
responsive `setSize` call. Run the focused frontend tests and observe the
expected failures before implementation.

### Task 4: Implement grip ownership and responsive native minimum height

**Files:**

- Modify: `src/components/shared/WindowGrip.tsx`
- Modify: `src/components/widget/WidgetHeader.tsx`
- Modify: `src/components/widget/TokenTracingWidget.tsx`
- Modify: `src/components/settings/SettingsScreen.tsx`
- Modify: `src/hooks/useSettingsController.ts`
- Modify: `src/lib/window-sizing.ts`
- Modify: `src/tests/components/shared/WindowControls.test.tsx`
- Modify: `src/tests/components/widget/TokenTracingWidget.test.tsx`
- Modify: `src/tests/components/settings/SettingsScreen.test.tsx`
- Modify: `src/tests/lib/window-sizing.test.ts`

Make `WindowGrip` call `startCurrentWindowDrag` on primary-button press with
default/propagation suppression and expose a keyboard-operable move label.
Mount it directly under each panel root, remove `data-tauri-drag-region` and
the settings header drag handler, and leave close/toggle/content controls
non-draggable. Update `syncWidgetWindowHeight` to set the native logical
constraints `360–720` wide and `targetHeight–520` high, checking the latest
request before applying size. Add the narrow Tauri permission required by
`setSizeConstraints`.

### Task 5: Remove the native perimeter and rebalance panel spacing

**Files:**

- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/app/tray.rs`
- Modify: `src-tauri/src/app/tray.rs` tests
- Modify: `src/styles/window-controls.css`
- Modify: `src/styles/widget.css`
- Modify: `src/styles/settings.css`

Disable native Tauri shadow for both transparent frameless windows while
retaining the CSS surface shadows. Style the grip as a small top-center
bordered pill with the six dots in its own hit target, raise its stacking order
above the top resize handle, and move the resize hit zones around it. Remove
the old header-grab cursor/state. Increase widget provider list/row padding,
allow provider content to remain visible, and keep the existing type roles,
colors, and dark/light tokens intact.

### Task 6: Update maintained context and verify the complete slice

**Files:**

- Modify: `CONTEXT.md`
- Inspect: `docs/superpowers/specs/2026-08-31-widget-polish-design-update.md`

Record the new grip ownership, dynamic minimum-height contract, no-native-
shadow decision, and day-scoped current-session meaning in the maintained
context. Then run, as applicable:

```text
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug
git diff --check
```

Run the Impeccable detector on changed UI files and review the full diff. A
Windows packaged-app smoke check must confirm: both panels have no perimeter
artifact; dragging starts only from the top-center grip; resize cannot hide
provider data; hiding a provider changes the widget's safe minimum/height; and
an event from yesterday shows current session `0` while retaining its relative
last-update context.

## Self-review checklist

- [ ] Tests fail for each new behavior before the corresponding implementation.
- [ ] No old `data-tauri-drag-region` or header-level drag path remains.
- [ ] Native size constraints and Tauri capability are consistent with the
  logical widget bounds.
- [ ] CSS shadow remains visible without a native transparent-window outline.
- [ ] Provider spacing is improved without changing the approved typography
  roles or introducing a font package.
- [ ] Current-day filtering changes only current-session values, not state,
  last-update metadata, source health, or today's totals.
- [ ] Frontend, Rust, integrated build, diff, detector, and Windows smoke
  evidence are reported separately as verified or pending.
