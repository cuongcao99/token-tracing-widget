# Lane B report

Date: 2026-09-01
Branch: `refactor/ui-ux`
Scope: settings activity and persistence isolation; no staging or commits

## Implemented

- Added `useSettingsActivity`, the only settings-side usage subscription. It
  derives the existing provider status and relative update labels from the
  current summary.
- Added `SettingsActivityPanel`. The panel calls the activity hook internally,
  renders both provider visibility and source settings, and uses the same
  activity summary for source health. `SettingsScreen` receives only editable
  settings values and callbacks, so Appearance remains outside activity
  updates.
- Extracted `useSettingsPersistence` with the existing complete preview
  payload, FIFO persistence chain, error conversion, pending-work tracking,
  close flush, and post-unmount queue guard. No debounce or coalescing was
  added. The native picker remains controller-owned and immediately applies
  its validated snapshot.
- Kept `useSettingsController` edit-only for usage data while preserving
  source loading, widget-settings synchronization, picker behavior, and all
  existing callback payload shapes.
- Added `data-theme` and `data-color-mode` to the settings root while retaining
  legacy classes for the later CSS Module pass.
- Split the old 351-line settings screen test into focused structure,
  behavior, activity, persistence, and render-isolation suites. Every touched
  or new Lane B TS/TSX/test file is below 250 lines.

## Changed files

Production:

- `src/hooks/useSettingsActivity.ts`
- `src/hooks/useSettingsPersistence.ts`
- `src/hooks/useSettingsController.ts`
- `src/components/settings/SettingsActivityPanel.tsx`
- `src/components/settings/SettingsScreen.tsx`

Tests:

- Deleted `src/tests/components/settings/SettingsScreen.test.tsx`
- `src/tests/components/settings/SettingsScreen.structure.test.tsx`
- `src/tests/components/settings/SettingsScreen.behavior.test.tsx`
- `src/tests/components/settings/SettingsActivityPanel.test.tsx`
- `src/tests/components/settings/SettingsScreen.render-isolation.test.tsx`
- `src/tests/hooks/useSettingsPersistence.test.ts`
- `src/tests/hooks/useSettingsController.edge.test.ts`

## Verification

Focused Lane B command:

```text
npm test -- --run src/tests/components/settings/SettingsScreen.structure.test.tsx src/tests/components/settings/SettingsScreen.behavior.test.tsx src/tests/components/settings/SettingsActivityPanel.test.tsx src/tests/components/settings/SettingsScreen.render-isolation.test.tsx src/tests/hooks/useSettingsPersistence.test.ts src/tests/hooks/useSettingsController.edge.test.ts
```

Result after the review fixes: 6 files, 20 tests passed.

Broader settings regression after the parallel stable facades were restored:

```text
npm test -- --run src/tests/hooks/useSettingsPersistence.test.ts src/tests/hooks/useSettingsController.edge.test.ts src/tests/components/settings
```

Result after adding the controller edge suite: 8 files, 24 tests passed. This
includes the existing settings model and appearance tests together with the
new activity, behavior, isolation, persistence, and controller edge coverage.
This command intentionally does not include the stable lib facade tests; those
are covered by the full frontend run below.

Full frontend suite after the parallel A/C changes were restored:

```text
npm test -- --run
```

Result: 27 files, 85 tests passed, including the widget settings/preview and
desktop facade tests.

Build:

```text
npm run build
```

Result: TypeScript and both Vite entries passed. The initial sandboxed attempts
hit the known Windows `esbuild spawn EPERM`; the checks were rerun in the
approved elevated process context. `git diff --check` passed for the touched
tracked Lane B files.

The render-isolation test drives a usage event through the activity listener
and confirms Appearance's render count does not change. Deferred tests prove
the second write waits for the first, `flush()` waits for the newest write and
preview, a failed write does not block the following FIFO write, errors reach
`onError`, and queued/new work is ignored after cleanup while started work
settles. Controller edge tests prove edits do not emit previews while source
settings are still loading and a cancelled native picker does not write.

## Remaining limits

- CSS remains global and legacy class names remain intentionally; D2 owns the
  settings CSS Module migration.
- Rust, Tauri debug packaging, and Windows native/visual smoke checks remain
  root integration work.
- `settings-model.ts` and all stable lib facades were left untouched by Lane B;
  C owns their compatibility reexport and desktop boundary changes.
