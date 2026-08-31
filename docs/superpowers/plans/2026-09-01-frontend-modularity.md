# Frontend Modularity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use the lane brief in `.superpowers/sdd/2026-09-01-frontend-modularity/` for the assigned lane. Workers edit only the files assigned to their lane, run the stated focused checks, and return the checkpoint to root. Root owns integration, staging, commits, and pushes.

**Goal:** Refactor the existing React frontend into provider/view-model,
settings activity/persistence, desktop-contract, and CSS Module boundaries while
preserving the current UI, runtime behavior, privacy boundary, and wire
contracts.

**Architecture:** Three disjoint TSX/contract lanes run in parallel after the
verified `dev` recovery checkpoint. Lane A centralizes provider branding and
maps the existing summary/settings data into composable widget primitives. Lane
B separates the usage subscription from settings editing and extracts the
existing preview/persistence queue. Lane C moves Tauri calls into a desktop
boundary and leaves pure validators/formatters behind stable `src/lib/*.ts`
facades. After those lanes are integrated, D1 and D2 migrate widget/shared and
settings styles to CSS Modules in sequence.

**Tech Stack:** React 19, TypeScript, Vite, Vitest, Testing Library, plain CSS
Modules, Tauri 2 APIs through the existing bridge, and the existing Windows
packaged smoke workflow. No new dependency is added.

**Spec:** `docs/superpowers/specs/2026-09-01-frontend-modularity-design-update.md`

## Reviewed preflight corrections

These corrections govern the illustrative interfaces and task descriptions below.
The implementation must preserve the signatures and behavior in current source.

- `SettingsActivityPanel` calls `useSettingsActivity` internally and renders both
  provider visibility and source settings. Neither `SettingsScreen` nor its edit
  controller subscribes to usage or receives the live summary.
- Persistence remains FIFO, without debounce, coalescing, or a source text editor.
  The native picker remains an immediate persistent operation owned by the controller.
- Lane C owns moving `components/settings/settings-model.ts` to
  `lib/settings-model.ts` and leaving a compatibility re-export. Lane B consumes
  that existing path without editing it. C also owns type-only `UnlistenFn` import
  replacements in `useUsageSummary.ts` and `useWidgetSettings.ts`.
- C preserves permissive optional top-level usage fields and the display
  `provider?: string`; nested provider IDs stay strict. Exact-key and allowed-key
  validators retain their distinct behavior.
- Each lane is reviewed and committed separately. No empty integration commit is
  needed. The spec and plan are committed before CSS migration; final documentation
  records evidence separately from unperformed native smoke checks.

## Global Constraints

- Work on `refactor/ui-ux`; the verified recovery checkpoint is
  `3395237e9ac89b35ecaaebbae174aa08d5aa02d0`, identical to `origin/dev`.
- The checkpoint intentionally has 52 passing frontend tests plus two failing
  style suites that reference the deleted static preview stylesheet. Task 0
  repairs those test references only; it does not recreate the preview or
  change production source.
- Keep version one Windows 11-only, local-only, and metadata-only.
- Do not change Rust, Tauri commands/events/capabilities, SQLite, collection,
  provider readers, source discovery, or frontend wire payloads.
- Preserve exact command names, event names, public facade signatures, error
  codes, DTO validators, provider IDs, settings behavior, window sizing, grip,
  native resize, picker, and current visual values.
- Do not add a provider, a theme, a session record/ID, a network client,
  telemetry, sidecar, background service, state library, CSS framework, ORM,
  font package, React Canary, or React Native.
- Use existing `src/lib/*.ts` facades during parallel work. Only
  `src/lib/desktop/*` may import Tauri APIs in production after Task C.
- Global CSS may contain only reset/base rules, semantic tokens, and scoped
  theme declarations. Component styles live in responsibility folders and do
  not use `var(--claude-...)` fallbacks or parent piercing.
- Root owns all Git mutations. Workers do not stage, commit, push, create
  worktrees, or spawn helpers.
- Every touched/new TS/TSX/CSS file, including tests, must stay at or below
  250 lines. Target 80–200 lines for responsibility modules; split larger
  current tests or styles by behavior rather than padding a file.
- Do not stage `.superpowers/`, `.impeccable/`, dependencies, build output,
  browser profiles, or local `.claude/` settings. Root may promote the two
  reviewed documents to the canonical `docs/superpowers/` paths.

## Exclusive ownership and handoff

| Lane | Exclusive files during the lane | Handoff condition |
| --- | --- | --- |
| Task 0 | `src/tests/styles/panel-surfaces.test.mjs`, `src/tests/styles/widget-rhythm.test.mjs` | Both tests read runtime styles only, no static preview path, and focused Vitest passes. Ownership transfers to D1/D2 for module assertions. |
| A | `src/lib/provider.ts`, new `src/lib/widget-view-model.ts`; `src/components/shared/ProviderBrand.tsx`, `ProviderDot.tsx`, `ProviderName.tsx`; new `src/components/widget/ProviderSection.tsx`, `UsageMetrics.tsx`; `ProviderUsageRow.tsx`, `TokenTracingWidget.tsx`, `widget-types.ts`; widget/view-model/branding tests | Focused A tests pass; root reviews no session object/ID or unknown provider path. Keep legacy class names until D1. |
| B | New `src/hooks/useSettingsActivity.ts`, `useSettingsPersistence.ts`; `useSettingsController.ts`; `src/components/settings/settings-model.ts`; all settings TSX under `src/components/settings/`; split settings component/hook tests | Focused B tests pass; root reviews queue/close/error and activity/edit separation. Keep legacy class names until D2. |
| C | New `src/lib/contracts/*`, `src/lib/desktop/*`; existing `src/lib/usage-summary.ts`, `widget-settings.ts`, `widget-settings-preview.ts`, `source-settings.ts`, `window-actions.ts`, `window-sizing.ts`; contract/facade tests | Focused C tests pass; `rg` proves production Tauri imports are confined to desktop; public facades remain import-compatible. |
| D1 | New `src/styles/globals/*`, `src/styles/widget/*`, `src/styles/shared/*`; widget/shared TSX style imports; `src/main.tsx`; `widget-rhythm` and generic CSS boundary tests; removal of old global widget/shared styles | A/B/C integrated and root review passes; widget bundle has only globals + widget/shared modules. |
| D2 | New `src/styles/settings/*`; settings TSX style imports; `src/settings-main.tsx`; `panel-surfaces` and settings/bundle isolation tests; removal of old global settings/index styles | D1 token/theme contract passes; settings bundle has only globals + settings/shared modules. |
| Root | Integration order, visual/native baseline artifacts, review, staging/commits, final checks, and promotion to `docs/superpowers/` | Each lane is independently reviewed before the next handoff. |

No two active workers edit a file in the same row. D1/D2 are sequential and
receive the TSX files only after A/B have finished their temporary legacy-class
phase.

## Checkpoint sequence

1. Root records the exact `dev` recovery checkpoint. Task 0 and the visual
   baseline worker run independently; the missing preview remains explicit.
2. Root dispatches A, B, and C in parallel against stable existing facades.
   Each lane runs focused tests and returns a reviewable diff without a commit.
3. Root reviews each lane, runs the cross-lane frontend suite/build, then
   creates one incremental integration checkpoint containing A, B, and C.
4. D1 migrates globals and widget/shared styles. Root reviews bundle isolation,
   CSS contracts, and widget smoke before D2 starts.
5. D2 migrates settings styles and per-window imports. Root reviews settings
   smoke and then runs all frontend/native verification.
6. Root records the final diff, line-count/import checks, packaged Windows
   smoke evidence, and promotes the reviewed spec/plan to their canonical
   paths. The original `dev` checkpoint remains a valid recovery point.

---

## Task 0: Repair the stale style-test baseline

**Files:**

- Modify: `src/tests/styles/panel-surfaces.test.mjs`
- Modify: `src/tests/styles/widget-rhythm.test.mjs`

**Owner:** `repair_style_baseline` (no production edits)

**Interfaces:**

- Consumes: current runtime files under `src/styles/` and
  `src/lib/widget-layout.ts`.
- Produces: style regression tests that do not read absent
  `src/design-preview.css`; D1/D2 later replace their selectors with module
  contracts.

- [ ] **Step 1: Confirm the intentional failure shape.**

Run:

```powershell
Test-Path src/design-preview.css
npm test -- --run src/tests/styles/panel-surfaces.test.mjs src/tests/styles/widget-rhythm.test.mjs
```

Expected: `False` for the deleted preview stylesheet and failures only from
the two stale preview reads. If Vite/esbuild reports `spawn EPERM`, rerun the
same command in the approved elevated context; do not patch application code.

- [ ] **Step 2: Remove only stale preview reads/assertions.**

Keep runtime assertions for settings scrollbar edge alignment, widget/settings
surface elevation, provider marks, theme picker selectors, rhythm, and current
192/520 layout constants. Remove `readStylesheet("../../design-preview.css")`,
`previewCss`, and assertions whose only subject is the missing static preview.
Do not add a replacement preview file or change `src/styles/*`.

- [ ] **Step 3: Run the focused repair check.**

Run:

```powershell
npm test -- --run src/tests/styles/panel-surfaces.test.mjs src/tests/styles/widget-rhythm.test.mjs
```

Expected: both suites pass. Report the resulting frontend count and the fact
that production files were untouched. Root commits this baseline checkpoint
before accepting the A/B/C lane diffs.

---

## Task 1: Lane A — provider registry, view model, and widget composition

**Files:**

- Modify: `src/lib/provider.ts`
- Create: `src/lib/widget-view-model.ts`
- Create: `src/components/shared/ProviderBrand.tsx`
- Modify: `src/components/shared/ProviderDot.tsx`
- Modify: `src/components/shared/ProviderName.tsx`
- Create: `src/components/widget/ProviderSection.tsx`
- Create: `src/components/widget/UsageMetrics.tsx`
- Modify: `src/components/widget/ProviderUsageRow.tsx`
- Modify: `src/components/widget/TokenTracingWidget.tsx`
- Modify: `src/components/widget/widget-types.ts`
- Modify: `src/tests/lib/provider.test.ts`
- Create: `src/tests/lib/widget-view-model.test.ts`
- Create: `src/tests/components/widget/ProviderSection.test.tsx`
- Modify: `src/tests/components/widget/TokenTracingWidget.test.tsx`
- Modify: `src/tests/components/shared/ProviderBranding.test.tsx`

**Interfaces:**

- Consumes: existing `src/lib/usage-summary.ts`, `widget-settings.ts`, and
  `widget-settings-preview.ts` facades. These imports remain unchanged while
  C extracts their implementation.
- Produces: a registry-driven `WidgetViewModel`, branding metadata, and
  `ProviderSection`/`UsageMetrics` composition consumed by D1.

The registry must expose the current Claude/Codex IDs and values plus imported
logo sources and generic logo variants. Use these types:

```ts
export type ProviderLogoVariant = "warm-mark" | "monochrome-mark";

export interface ProviderIdentity {
  name: string;
  displayName: string;
  logoSrc: string;
  logoVariant: ProviderLogoVariant;
  fontRole: "display" | "ui";
  accent: string;
}

export interface ProviderRegistration extends ProviderIdentity {
  id: ProviderId;
  automaticRoot: string;
  displayRoot: string;
}
```

`ProviderDot` and `ProviderName` may remain as thin compatibility wrappers for
existing tests, but they must delegate to `ProviderBrand` metadata and contain
no provider-name branch. The final CSS receives generic `data-logo-variant`,
`data-font-role`, and `--provider-accent` values.

`WidgetViewModel` has no session object or session ID:

```ts
export interface WidgetViewModelInput {
  summary: UsageSummary;
  settings: WidgetSettingsSnapshot;
  previewSourceEnabled: Readonly<Record<ProviderId, boolean>> | null;
}

export interface WidgetProviderViewModel {
  provider: ProviderId;
  identity: ProviderIdentity;
  status: { state: UsageState; label: string };
  metrics: {
    sessionTokens?: number;
    todayTokens: number;
    updatedLabel: string;
  };
}

export interface WidgetViewModel {
  theme: ThemeId;
  colorMode: "dark" | "light";
  providers: WidgetProviderViewModel[];
  totalTokens: number;
  visibleProviderCount: number;
}

export function createWidgetViewModel(
  input: WidgetViewModelInput,
): WidgetViewModel;
```

`ProviderSection` takes `identity`, `status`, and `children: ReactNode`.
`UsageMetrics` takes `readonly UsageMetric[]` and `updatedLabel`. A current
row passes Session and Today metrics; a future individual-session child can be
added without changing the wire contract. Its identity type intentionally has
no runtime provider ID, so a third provider composition fixture can test the
shell without registering or accepting a third ID.

- [ ] **Step 1: Write failing view-model and composition tests.**

Add tests that a two-provider summary maps in registry order, hidden providers
are omitted, preview-disabled providers render as unavailable and are removed
from the preview total, and loading/active/idle/unavailable/stale labels remain
unchanged. Add a fixture-only `ProviderIdentity` object with no runtime ID to
`ProviderSection.test.tsx`, render three nested metric values, and assert the
children remain intact. Add branding assertions that both registry logo
sources render and no `Claude Code` long label leaks into the visible name.

Run:

```powershell
npm test -- --run src/tests/lib/widget-view-model.test.ts src/tests/components/widget/ProviderSection.test.tsx src/tests/components/widget/TokenTracingWidget.test.tsx src/tests/components/shared/ProviderBranding.test.tsx
```

Expected: new tests fail because the view-model and composition primitives do
not exist yet.

- [ ] **Step 2: Implement the registry and pure mapper.**

Move the two existing logo imports into `provider.ts`, add generic metadata,
and derive `providerOrder`, `providerMeta`, and `isProviderId` from the same
registry. In `createWidgetViewModel`, iterate the registry, read matching
summary entries, preserve `formatTokens`/`formatRelativeUpdate` behavior,
filter by the settings visibility map, mark a preview-disabled source
unavailable, and calculate the same combined total. Keep root data attributes
on the widget while retaining legacy classes until D1.

- [ ] **Step 3: Implement the composition primitives and adapter.**

`ProviderSection` renders the existing heading/status shell and its children.
`UsageMetrics` renders the existing metric grid and relative update label.
`ProviderUsageRow` adapts one `WidgetProviderViewModel` into those primitives;
`TokenTracingWidget` calls the mapper, passes `visibleProviderCount` to the
existing sizing facade, and renders mapped rows. Do not add a session record,
session count, or new payload field.

- [ ] **Step 4: Run the lane checks.**

Run:

```powershell
npm test -- --run src/tests/lib/provider.test.ts src/tests/lib/widget-view-model.test.ts src/tests/components/widget/ProviderSection.test.tsx src/tests/components/widget/TokenTracingWidget.test.tsx src/tests/components/shared/ProviderBranding.test.tsx
npm run build
```

Expected: PASS, with both Vite entries building. Return the diff to root; do
not stage or commit.

---

## Task 2: Lane B — settings activity and persistence isolation

**Files:**

- Create: `src/hooks/useSettingsActivity.ts`
- Create: `src/hooks/useSettingsPersistence.ts`
- Modify: `src/hooks/useSettingsController.ts`
- Modify: `src/components/settings/settings-model.ts` for pure edit/view
  transforms and the existing platform-aware error copy
- Create: `src/components/settings/SettingsActivityPanel.tsx`
- Modify: all settings TSX under `src/components/settings/` as needed to keep
  the same structure and temporary legacy class names
- Delete: `src/tests/components/settings/SettingsScreen.test.tsx` after its
  behavior is split
- Create: `src/tests/components/settings/SettingsScreen.structure.test.tsx`
- Create: `src/tests/components/settings/SettingsScreen.behavior.test.tsx`
- Create: `src/tests/components/settings/SettingsActivityPanel.test.tsx`
- Create: `src/tests/hooks/useSettingsPersistence.test.ts`
- Modify: `src/tests/components/settings/AppearanceSection.test.tsx`

**Interfaces:**

- Consumes: existing settings/source/usage facades and the pure helpers from
  `src/components/settings/settings-model.ts`. C preserves the facade imports
  and supplies `desktop/platform-copy.ts` for the existing copy constants.
- Produces: activity data independent of edit state and the persistence hook
  consumed by `useSettingsController`.

Use these interfaces without adding a global store:

```ts
export interface SettingsActivity {
  summary: UsageSummary;
  providerStatuses: ProviderStatusView[];
}

export function useSettingsActivity(): SettingsActivity;

export interface UseSettingsPersistenceResult {
  sendPreview(preview: WidgetSettingsPreview): void;
  saveWidget(snapshot: WidgetSettingsSnapshot): void;
  saveSource(settings: SourceSettings): void;
  flush(): Promise<void>;
}

export function useSettingsPersistence(
  onError: (message: string) => void,
): UseSettingsPersistenceResult;
```

`useSettingsActivity` is the only settings-side usage subscription. It calls
the existing `useUsageSummary`, derives provider statuses with the current
relative-time formatter, and is called inside `SettingsActivityPanel`. The panel
renders `ProviderVisibilitySection` and `SourceSettingsSection` using that same
summary, and receives only editable values and callbacks from the controller.
It does not own edit state or send activity data back to `SettingsScreen`.

`useSettingsPersistence` keeps the current pending preview promise and
serialized persistence chain. `saveWidget` and `saveSource`
must preserve exact payload shapes and command error codes. Preview emits the
complete current preview. `flush()` waits for both queues. Cleanup prevents new
work after unmount while an already-started native operation may settle.
Closing waits for `flush()` and then calls `closeCurrentWindow`; it never emits
an old snapshot. This extraction adds no debounce or coalescing policy.

- [ ] **Step 1: Split tests and write queue/activity failures.**

Move the current large settings test into structure and behavior files. The
structure file checks headings, close/grip/resize controls, a separate
scrolling body, root `data-theme="claude"`, and
`data-color-mode="dark"`/`light`. The behavior file checks visibility/source/
theme/dark-mode independence, exact preview and persistence payloads, picker
updates, errors, close flush, and no Save button. The activity test feeds
loading, active, idle, unavailable, and stale summaries and asserts each
status/relative update label. The persistence hook test uses deferred promises
to prove writes serialize, the newest snapshot is not
overtaken, errors reach `onError`, `flush()` waits, and cleanup ignores new
work after unmount.

Run:

```powershell
npm test -- --run src/tests/components/settings/SettingsScreen.structure.test.tsx src/tests/components/settings/SettingsScreen.behavior.test.tsx src/tests/components/settings/SettingsActivityPanel.test.tsx src/tests/hooks/useSettingsPersistence.test.ts
```

Expected: the new hook/activity tests fail while the old controller still
owns all concerns. Do not change lib facades in this lane.

- [ ] **Step 2: Extract `useSettingsPersistence`.**

Move `pendingPreview`, `pendingPersistence`, `sendPreview`, queue chaining,
error conversion, and close waiting into the hook. Keep `emitWidgetSettingsPreview`,
`updateWidgetSettings`, and `updateSourceSettings` calls behind their existing
facades. Keep the native picker path in the controller because the command
persists and returns a validated source snapshot.

- [ ] **Step 3: Extract activity and keep controller edit-only.**

Remove `useUsageSummary` and provider-status derivation from
`useSettingsController`. Add `useSettingsActivity` and
`SettingsActivityPanel`; have `SettingsScreen` pass editable values and callbacks
to the panel, which owns the subscription and source-health rendering. Keep root
legacy classes plus the new `data-theme` and `data-color-mode` attributes for
D2. Stabilize only callbacks/objects that cross a memoized child; do not add a
memoization library or broad `useMemo` wrappers.

- [ ] **Step 4: Run the lane checks.**

Run:

```powershell
npm test -- --run src/tests/components/settings src/tests/hooks/useSettingsPersistence.test.ts src/tests/lib/widget-settings.test.ts src/tests/lib/widget-settings-preview.test.ts
npm run build
```

Expected: PASS with exact existing settings behavior. Return the diff to root
without staging or committing.

---

## Task 3: Lane C — desktop boundary and stable contract facades

**Files:**

- Create: `src/lib/contracts/validation.ts`
- Create: `src/lib/contracts/usage-summary.ts`
- Create: `src/lib/contracts/widget-settings.ts`
- Create: `src/lib/contracts/widget-settings-preview.ts`
- Create: `src/lib/contracts/source-settings.ts`
- Create: `src/lib/desktop/commands.ts`
- Create: `src/lib/desktop/events.ts`
- Create: `src/lib/desktop/window.ts`
- Create: `src/lib/desktop/platform-copy.ts`
- Modify: `src/lib/usage-summary.ts`
- Modify: `src/lib/widget-settings.ts`
- Modify: `src/lib/widget-settings-preview.ts`
- Modify: `src/lib/source-settings.ts`
- Modify: `src/lib/window-actions.ts`
- Modify: `src/lib/window-sizing.ts`
- Modify: `src/tests/lib/usage-summary.test.ts`
- Modify: `src/tests/lib/widget-settings.test.ts`
- Modify: `src/tests/lib/widget-settings-preview.test.ts`
- Modify: `src/tests/lib/source-settings.test.ts`
- Modify: `src/tests/lib/window-sizing.test.ts`
- Create: `src/tests/lib/desktop-boundary.test.ts`
- Create: `src/tests/lib/import-boundaries.test.mjs`

**Interfaces:**

- Consumes: current provider registry and current facade signatures.
- Produces: pure contract modules plus desktop transport modules. Existing
  imports remain valid for A and B; no component changes are required in this
  lane.

The public facade signatures remain exactly:

```ts
getUsageSummary(): Promise<UsageSummary>;
listenForUsageSummary(onSummary: (summary: UsageSummary) => void): Promise<UnlistenFn>;
getWidgetSettings(): Promise<WidgetSettingsSnapshot>;
updateWidgetSettings(settings: WidgetSettingsSnapshot): Promise<WidgetSettingsSnapshot>;
listenForWidgetSettings(onSettings: (settings: WidgetSettingsSnapshot) => void): Promise<UnlistenFn>;
emitWidgetSettingsPreview(preview: WidgetSettingsPreview): Promise<void>;
listenForWidgetSettingsPreview(onPreview: (preview: WidgetSettingsPreview) => void): Promise<UnlistenFn>;
getSourceSettings(): Promise<SourceSettingsSnapshot>;
pickSourceRoot(provider: ProviderId): Promise<SourceSettingsSnapshot | null>;
updateSourceSettings(settings: SourceSettings): Promise<SourceSettingsSnapshot>;
startCurrentWindowDrag(): Promise<void>;
startCurrentWindowResize(direction: WindowResizeDirection): Promise<void>;
closeCurrentWindow(): Promise<void>;
syncWidgetWindowHeight(visibleProviderCount: number): Promise<void>;
```

`desktop/commands.ts` keeps the six exact command strings. `desktop/events.ts`
keeps the three exact event strings. `desktop/window.ts` keeps native drag,
resize, close, logical size, width clamp, and current size constraints. The
desktop files are the only production imports of `@tauri-apps/api`.

`contracts/*` contains the current safe DTO types, parsers, and formatters.
Keep permissive optional-field handling and exact-key rules as they are now;
do not tighten the top-level `UsageSummary.provider` behavior or alter error
strings. C exposes `desktop/platform-copy.ts`; B's settings-model consumes it
when it owns the existing Windows/approved-WSL and native-settings copy.

- [ ] **Step 1: Add boundary tests before moving code.**

Mock `@tauri-apps/api/core`, `event`, and `window` and assert each facade calls
the exact command/event string and payload shape. Add payload tests for unsafe
fields (`prompt`, `rawRecord`, arbitrary source roots), unknown/duplicate/
missing provider IDs, invalid themes, negative/unsafe token counts, and invalid
dates. Assert a valid current summary/settings/preview still round trips.
Add the source import test:

```ts
expect(contractSource).not.toContain("@tauri-apps/api");
expect(nonDesktopProductionSources).not.toMatch(/@tauri-apps\/api/);
```

Run:

```powershell
npm test -- --run src/tests/lib/desktop-boundary.test.ts src/tests/lib/usage-summary.test.ts src/tests/lib/widget-settings.test.ts src/tests/lib/widget-settings-preview.test.ts src/tests/lib/source-settings.test.ts
```

Expected: boundary tests fail because raw calls and validation currently share
the same files.

- [ ] **Step 2: Move pure validators and formatters.**

Extract shared record/key/token helpers into `contracts/validation.ts`; move
each DTO parser and `formatRelativeUpdate` into its contract module. Preserve
the current parser output normalization and rejection conditions. Do not add a
new wire field or a third-provider fallback.

- [ ] **Step 3: Add desktop calls and restore compatibility facades.**

Implement exact command/event/window functions under `desktop/`, then make
the existing `src/lib/*.ts` files re-export contract types/parsers/formatters
and delegate their public functions to desktop calls. Keep
`window-sizing.ts` as the stable sizing facade around the desktop window
transport and existing layout constants. Leave settings edit/view transforms
owned by B in `src/components/settings/settings-model.ts`.

- [ ] **Step 4: Run the lane checks.**

Run:

```powershell
npm test -- --run src/tests/lib
npm run build
rg -n "@tauri-apps/api" src/lib src/components src/hooks
```

Expected: all production matches are under `src/lib/desktop/`; all facade and
contract tests pass; both Vite entries build. Return the diff to root without
staging or committing.

---

## Task 4: Root integration and review checkpoint

**Files:** No worker source files; root reviews the A/B/C diffs and owns the
incremental integration commit.

**Interfaces:**

- Consumes: passing Task 0, A, B, and C focused checks.
- Produces: one integrated frontend TSX/contract checkpoint with legacy
  component class names still present and no CSS Module migration yet.

- [ ] **Step 1: Review ownership and public boundaries.**

Confirm A/B import only stable `src/lib/*.ts` facades, C has no component
imports to rewrite, and no lane changed `src-tauri/`, `package.json`, or
static preview files. Confirm `TokenTracingWidget` uses only the pure mapper,
`SettingsScreen` has a separate activity source, and only the persistence hook
owns pending queues/close flush.

- [ ] **Step 2: Run the integrated frontend check.**

Run:

```powershell
npm test -- --run
npm run build
git diff --check
```

Expected: the repaired baseline suites and all A/B/C tests pass. Root records
any failure with its owning lane before staging the integration checkpoint.

- [ ] **Step 3: Stage only the reviewed A/B/C files.**

Root stages each lane as an incremental commit after its review, then an
integration checkpoint. Do not stage `.superpowers/` or generated output.

---

## Task 5: D1 — global tokens, widget/shared CSS Modules, and widget bundle

**Files:**

- Create: `src/styles/globals/reset.css`
- Create: `src/styles/globals/tokens.css`
- Create: `src/styles/globals/themes.css`
- Create: `src/styles/widget/surface.module.css`
- Create: `src/styles/widget/provider.module.css`
- Create: `src/styles/widget/metrics.module.css`
- Create: `src/styles/widget/total.module.css`
- Create: `src/styles/shared/branding.module.css`
- Create: `src/styles/shared/window-controls.module.css`
- Modify: widget/shared TSX files owned by A only to import module objects
- Modify: `src/main.tsx`
- Delete or leave unused after verification: `src/styles/index.css`,
  `src/styles/base.css`, `src/styles/widget.css`,
  `src/styles/window-controls.css`
- Modify: `src/tests/styles/widget-rhythm.test.mjs`
- Create: `src/tests/styles/css-boundary.test.mjs`

**Interfaces:**

- Consumes: A's metadata-driven branding and composition, B's widget root
  `data-theme`/`data-color-mode`, and C's stable bridge imports.
- Produces: local CSS Module class maps and global semantic tokens consumed by
  widget/shared TSX and D2 settings modules.

Keep all current computed values: Claude colors, system font stacks, type
roles, 4px spacing rhythm, 8/12/16px radii, grip/resize dimensions and
cursors, 192/244/316 widget targets, shadow-free widget, and focus behavior.
The global token files define semantic slots for color, type, spacing, radius,
elevation, focus, window padding, and responsive rhythm. Theme declarations
are scoped to `[data-theme="claude"]` and `[data-color-mode="light"]` or
`[data-color-mode="dark"]`; component modules consume semantic variables with
no `var(--claude-...)` fallback.

- [ ] **Step 1: Add failing module and isolation tests.**

Assert every widget/shared module is under the correct folder, contains no
provider-named selector or `var(--claude-` fallback, and uses local selectors.
Assert `main.tsx` imports globals, widget modules, and shared controls while
not importing settings modules. Keep a rhythm test against the widget module
selectors and current `widget-layout.ts` constants.

- [ ] **Step 2: Create semantic globals and split styles by responsibility.**

Move the reset/base rules to `globals/reset.css`; expand `globals/tokens.css`
with the complete current semantic token set; put only theme declarations in
`globals/themes.css`. Split the old widget and window-control rules into the
four widget modules and two shared modules. Preserve selector ownership: the
provider module styles `ProviderSection`/`UsageMetrics`, branding styles only
branding, and window controls style only grip/resize.

- [ ] **Step 3: Replace widget/shared class strings with module maps.**

Import each module in its owning component and compose only local classes.
Keep generic `data-logo-variant`, `data-font-role`, and inline accent custom
property attributes from A. Remove provider-specific class names and old
`theme--*`/`widget--*` style branches while retaining root data attributes.
Do not use a parent selector to style a child component.

- [ ] **Step 4: Run D1 checks.**

Run:

```powershell
npm test -- --run src/tests/styles/widget-rhythm.test.mjs src/tests/styles/css-boundary.test.mjs src/tests/components/widget src/tests/components/shared
npm run build
```

Expected: widget/shared tests pass and the widget bundle builds without
settings CSS. Root reviews a browser render before D2 starts.

---

## Task 6: D2 — settings CSS Modules and per-window isolation

**Files:**

- Create: `src/styles/settings/surface.module.css`
- Create: `src/styles/settings/forms.module.css`
- Create: `src/styles/settings/theme-picker.module.css`
- Modify: settings TSX files owned by B to import module objects
- Modify: `src/settings-main.tsx`
- Delete or leave unused after verification: `src/styles/settings.css`
- Modify: `src/tests/styles/panel-surfaces.test.mjs`
- Create: `src/tests/styles/settings-modules.test.mjs`
- Create: `src/tests/styles/window-bundle-isolation.test.mjs`

**Interfaces:**

- Consumes: D1 globals/themes/shared controls, B's settings activity/edit
  structure, and C's pure settings model.
- Produces: settings-only module class maps and a bundle-isolated settings
  entry.

Keep the sticky header/body split, scrollbar edge extension, stable gutter,
hidden WebKit arrows, 440–820 by 420–900 settings bounds, 600px default,
borderless surface, diffuse light/dark elevation, theme picker, switch, close
button, and focus rings exactly as currently rendered.

- [ ] **Step 1: Add failing settings/module tests.**

Assert settings modules contain no provider-specific selectors, no Claude
fallbacks, and no parent piercing. Assert `settings-main.tsx` imports globals,
settings modules, and shared controls but no widget modules. Preserve runtime
scrollbar/elevation checks from Task 0 against the new module source.

- [ ] **Step 2: Split settings styles by responsibility.**

Move root/body/header/status/card/row styles to `surface.module.css`, switches,
source rows, and status styles to `forms.module.css`, and the custom theme
picker styles to `theme-picker.module.css`. Consume D1 semantic tokens only;
do not duplicate raw Claude values or add a second theme.

- [ ] **Step 3: Replace settings class strings and update the entry import.**

Import module maps in each settings component, keep the existing accessible
roles/labels, and leave `data-theme`/`data-color-mode` on the settings root.
Use the shared module for grip/resize and keep all native event ownership
unchanged. Remove the shared `index.css` import so the two Vite entries have
disjoint component style graphs.

- [ ] **Step 4: Verify bundle isolation.**

Run:

```powershell
npm test -- --run src/tests/styles/panel-surfaces.test.mjs src/tests/styles/settings-modules.test.mjs src/tests/styles/window-bundle-isolation.test.mjs src/tests/components/settings
npm run build
```

Expected: both entry bundles build; source/import checks prove the widget does
not pull settings modules and Settings does not pull widget modules. Root
reviews the generated bundle only as verification output and does not stage it.

---

## Task 7: Final verification, Windows smoke, and document promotion

**Files:**

- Inspect: full diff and generated build output (do not stage output)
- Promote reviewed working copies to:
  `docs/superpowers/specs/2026-09-01-frontend-modularity-design-update.md`
  and `docs/superpowers/plans/2026-09-01-frontend-modularity.md`

**Interfaces:**

- Consumes: D1/D2 passing checks and root's visual baseline artifacts.
- Produces: verified refactor handoff with separate automated and manual
  evidence.

- [ ] **Step 1: Run the complete automated gate.**

Run:

```powershell
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug
git diff --check
```

Expected: all frontend suites, including repaired style contracts, pass;
frontend build and existing Rust/package gates pass; `git diff --check` is
clean. If frontend/esbuild is blocked by `spawn EPERM`, use the approved
elevated execution context and record that environment detail.

- [ ] **Step 2: Run source hygiene and line-budget checks.**

Run:

```powershell
rg -n "@tauri-apps/api" src/lib src/components src/hooks
rg -n "var\(--claude-|provider-(?:name|dot)--(?:claude|codex)|\.provider-(?:claude|codex)" src/styles src/components
Get-ChildItem src -Recurse -File -Include *.ts,*.tsx,*.css,*.mjs | ForEach-Object { $count = (Get-Content $_.FullName).Count; if ($count -gt 250) { "OVER 250: $($_.FullName) $count" } }
git status --short
```

Expected: only `src/lib/desktop/*` has production Tauri imports; no module
contains forbidden Claude fallback/provider-specific selectors; no touched
TS/TSX/CSS file exceeds 250 lines; no generated output or `.superpowers/`
content is staged.

- [ ] **Step 3: Perform packaged Windows smoke.**

The 16-screen visual baseline under `.impeccable/frontend-modularity/visual/`
comes from an automatic synthetic Tauri harness with mocked bridge state. Use
it to compare React entry-point rendering and detect visual drift, but do not
count it as Windows, native-window, or DPI evidence. Launch the debug package
for the native check and verify both windows render with the current
Claude values and system fallback stacks. Verify the widget root and settings
root expose `data-theme="claude"` and the correct `data-color-mode`; hiding a
provider still requests 192/244/316 targets; the six-dot grip alone starts
native drag; all eight native resize handles work; settings body scrolls with
the header fixed; the picker, preview, auto-save errors, and close flush work;
the widget has no perimeter shadow and Settings retains diffuse elevation.
Confirm no session IDs, source paths outside the existing display labels, or
raw provider content appears in rendered DOM or bridge payloads.

- [ ] **Step 4: Promote documents and report evidence.**

Root copies the reviewed scratch spec and plan to the canonical
`docs/superpowers/` paths, commits them with the final documentation change,
and reports baseline repair, A/B/C integration, D1/D2 CSS migration,
automated gates, package result, and manual smoke results separately. Do not
claim a static preview, a new provider/theme, or Linux/macOS support.
