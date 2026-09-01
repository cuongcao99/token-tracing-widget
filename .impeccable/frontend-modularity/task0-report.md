# Task0 report: runtime style contracts

## Scope

Updated the two style contract suites so they read runtime styles only. Removed
the deleted `src/design-preview.css` imports and preview-only assertions. No
preview stylesheet or script was restored, and no production CSS or JavaScript
was changed.

## Changed files

- `src/tests/styles/panel-surfaces.test.mjs`
- `src/tests/styles/widget-rhythm.test.mjs`

## Verification

- Baseline: `npm test -- --run src/tests/styles/panel-surfaces.test.mjs src/tests/styles/widget-rhythm.test.mjs`
  - Expected red baseline reproduced in elevated Windows context: 2 failed
    suites during collection with `ENOENT` for `src/design-preview.css`; 0
    tests ran.
- Focused post-change: `npm test -- --run src/tests/styles/panel-surfaces.test.mjs src/tests/styles/widget-rhythm.test.mjs`
  - 2 files passed, 5 tests passed.
- Full frontend suite: `npm test -- --run`
  - 18 files passed, 57 tests passed.

The initial sandboxed baseline could not start Vitest because esbuild returned
Windows `spawn EPERM`; the baseline and verification commands were rerun with
the approved elevated process context.

## Self-review and limitations

- Preserved all runtime assertions covering settings scrollbar placement,
  settings/widget surface treatment, provider branding, and widget rhythm.
- Removed only assertions whose selectors or contracts belonged to the
  deleted static preview surface.
- This task validates source-level CSS contracts; it does not replace the
  separate Windows packaged visual smoke check.
