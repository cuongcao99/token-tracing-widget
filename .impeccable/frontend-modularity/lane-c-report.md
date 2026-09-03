# Lane C report — desktop boundary and contract facades

Date: 2026-09-01
Owner: `implement_desktop_contracts`

## Owned files

Created pure contracts and desktop boundary modules:

- `src/lib/contracts/validation.ts`
- `src/lib/contracts/usage-summary.ts`
- `src/lib/contracts/widget-settings.ts`
- `src/lib/contracts/widget-settings-preview.ts`
- `src/lib/contracts/source-settings.ts`
- `src/lib/desktop/commands.ts`
- `src/lib/desktop/events.ts`
- `src/lib/desktop/window.ts`
- `src/lib/desktop/platform-copy.ts`
- `src/lib/settings-model.ts`
- `src/tests/lib/desktop-boundary.test.ts`
- `src/tests/lib/import-boundaries.test.mjs`

Updated compatibility facades and the requested type-only hook imports:

- `src/lib/usage-summary.ts`
- `src/lib/widget-settings.ts`
- `src/lib/widget-settings-preview.ts`
- `src/lib/source-settings.ts`
- `src/lib/window-actions.ts`
- `src/lib/window-sizing.ts`
- `src/components/settings/settings-model.ts`
- `src/hooks/useUsageSummary.ts`
- `src/hooks/useWidgetSettings.ts`

No Rust, command/event implementation, provider registry, widget, settings
component, style, package, or Git file was changed by this lane.

## Behavior and boundary checks

- Existing command names remain `get_usage_summary`, `get_widget_settings`,
  `update_widget_settings`, `get_source_settings`, `pick_source_root`, and
  `update_source_settings` with the original payload shapes.
- Existing event names remain `usage-summary-changed`,
  `widget-settings-changed`, and `widget-settings-preview-changed`.
- Native drag, resize, close, responsive 192/244/316 heights, 360–720 width
  clamp, 192–520 height constraints, and serialized resize queue behavior were
  moved intact behind `desktop/window.ts`.
- Contract parsers retain permissive optional top-level summary fields and
  arbitrary display `provider?: string`, strict nested registered IDs, and the
  existing `hasOnlyKeys`/`hasExactKeys` distinction. Unsafe fields, duplicate
  or unknown IDs, invalid themes, negative/unsafe token counts, invalid dates,
  and invalid source root value types are rejected as before.
- `src/lib/settings-model.ts` owns pure edit/view transforms and existing
  error copy; the component path is a compatibility re-export.

## Focused verification

Command:

```text
npm test -- --run src/tests/lib/desktop-boundary.test.ts src/tests/lib/import-boundaries.test.mjs src/tests/lib/usage-summary.test.ts src/tests/lib/widget-settings.test.ts src/tests/lib/widget-settings-preview.test.ts src/tests/lib/source-settings.test.ts src/tests/lib/window-sizing.test.ts src/tests/components/settings/settings-model.test.ts
```

Result: PASS — 8 suites, 27 tests.

Command:

```text
npm test -- --run src/tests/lib
```

Result: PASS — 11 suites, 39 tests.

Command:

```text
npm run build
```

Result: PASS — `tsc -b` and Vite built both `index.html` and
`settings.html`. Build was run in the approved elevated context because the
normal sandbox hit the known esbuild `spawn EPERM` startup failure.

## Source import fence

Command:

```text
rg -n "@tauri-apps/api" src/lib src/components src/hooks
```

Result: exactly three production matches, all in:

- `src/lib/desktop/commands.ts`
- `src/lib/desktop/events.ts`
- `src/lib/desktop/window.ts`

`src/lib/contracts/*` and all non-desktop production sources contain no Tauri
imports. No source-level import abstraction or new transport was introduced.

## Line budget

Every touched/new TypeScript, TSX, CSS, and test file remains at or below the
250-line limit. The largest Lane C file is `src/lib/contracts/usage-summary.ts`
at 175 lines; the new boundary test is 221 lines.

Root owns review, staging, commits, and integration. This lane did not stage,
commit, push, or alter Git state.
