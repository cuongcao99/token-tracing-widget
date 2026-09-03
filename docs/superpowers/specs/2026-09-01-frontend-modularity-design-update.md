# Frontend Modularity, CSS Modules, and Desktop Boundary

**Date:** 2026-09-01
**Status:** Implementation-ready design update
**Canonical promotion paths:** `docs/superpowers/specs/2026-09-01-frontend-modularity-design-update.md` and `docs/superpowers/plans/2026-09-01-frontend-modularity.md`
**Working copy:** `.superpowers/sdd/2026-09-01-frontend-modularity/`

## Decision gate

The recovery checkpoint is `3395237e9ac89b35ecaaebbae174aa08d5aa02d0` on
`refactor/ui-ux`, and it is identical to `origin/dev`. It intentionally
contains the current implementation and its known baseline test condition:
52 frontend tests pass and the two style suites fail because the deleted
static preview stylesheet is still referenced. Native verification already
passed with 112 Rust tests, formatting, compile, and debug packaging. The
style baseline repair is a separate Task 0 and must not alter production
source or recreate the obsolete preview surface.

The refactor begins only after that checkpoint. It is a frontend-only change;
Rust, Tauri command implementations, SQLite, collection, provider readers,
source discovery, and the wire payloads are unchanged.

## Intent

The current widget and settings surfaces work, but their frontend boundaries
mix provider metadata with rendering, combine settings activity with edit
state, and import Tauri APIs beside DTO validation and formatters. The CSS is
global and shares both windows' selectors. This update separates those
responsibilities while keeping the current Claude presentation and runtime
behavior intact.

The resulting frontend is ready for a future individual-session composition
inside a provider section and for future themes that replace semantic tokens.
Those extension seams are type and composition boundaries only. This slice
does not add session records, session identities, providers, themes, or
platform implementations.

## Goals

- Keep one strict built-in provider registry as the source of display metadata,
  logo assets, branding variants, font roles, accents, and source labels.
- Map the existing `UsageSummary`, `WidgetSettingsSnapshot`, and preview state
  into a pure `WidgetViewModel` before rendering.
- Compose each provider as `ProviderSection(identity, status, children)` with
  `UsageMetrics` as a reusable metrics primitive. Current children contain only
  Session, Today, and the existing relative update label.
- Separate the settings usage subscription into a `SettingsActivityPanel`
  boundary and leave provider/source/theme edit state in the controller.
- Move preview, serialized persistence, pending work, errors, and close flush
  into `useSettingsPersistence` without changing their existing semantics.
- Isolate Tauri command/event/window calls and Windows-specific copy under
  `src/lib/desktop/`, while keeping pure DTO validators and formatters free of
  Tauri imports.
- Preserve existing import paths and signatures in `src/lib/*.ts` so widget
  and settings workers can proceed in parallel.
- Replace global component selectors with CSS Modules under responsibility
  folders for widget, settings, and shared controls. Keep globals limited to
  reset, semantic tokens, and theme declarations.
- Make each window import only its global foundation and the modules it uses.

## Non-goals

- No Rust or Tauri capability/configuration changes.
- No new Tauri command, event, DTO field, session record, session ID, or
  provider ID in a frontend payload.
- No dynamic plugin loading, arbitrary runtime provider IDs, or third provider
  implementation. A third provider is test fixture data for composition only.
- No second visual theme or theme editor. `claude` remains the only
  registered `ThemeId`.
- No change to the approved layout, colors, typography, spacing, elevation,
  responsive sizes, native grip/resize behavior, picker behavior, or settings
  preview/autosave/error/close behavior.
- No static design-preview recreation. The runtime React/Tauri surfaces and
  browser/native smoke checks are the source of visual evidence.
- No network client, telemetry, sidecar, background service, state library,
  CSS framework, ORM, font package, React Canary, or React Native.
- No Linux or macOS implementation. The boundary may use neutral names and
  avoid Windows assumptions outside the existing platform copy, but version
  one remains Windows 11-only.

## Existing behavior to preserve

| Area | Contract that remains unchanged |
| --- | --- |
| Provider IDs | Runtime IDs are exactly the registered Claude and Codex IDs. Unknown IDs are rejected by existing parsers. Registry order remains Claude then Codex. |
| Usage | Loading, active, idle, unavailable, and stale states remain readable. Provider current-session and Today values, combined Today total, and relative last update retain their current values. |
| Privacy | Prompts, responses, reasoning, tool payloads, credentials, repository contents, working directories, raw provider records, session keys, and arbitrary file contents stay outside normalized frontend data. |
| Settings | Provider visibility, source enabled state, source picker, theme, dark mode, immediate preview, serialized auto-save, inline errors, and close flush keep their current semantics. |
| Windows | Frameless transparent taskbar-hidden non-topmost windows, native six-dot grip, native eight-direction resize handles, current size bounds, and responsive widget targets remain unchanged. |
| Visuals | Claude semantic values and system font stacks remain the same. The widget is shadow-free; Settings keeps its diffuse CSS elevation. |

The widget size policy is still 192/244/316 logical pixels for zero, one, and
two visible providers, with existing 360–720 width and 192–520 height bounds.
The settings window keeps its current 440–820 width, 420–900 height, and 600px
default height policy. These are implementation contracts, not persisted data.

## Provider and branding boundary

`src/lib/provider.ts` remains the canonical registry and gains the existing
logo assets and generic branding variant metadata. A registration keeps a
strict `ProviderId` and includes at least:

```ts
type ProviderLogoVariant = "warm-mark" | "monochrome-mark";

interface ProviderIdentity {
  name: string;
  displayName: string;
  logoSrc: string;
  logoVariant: ProviderLogoVariant;
  fontRole: "display" | "ui";
  accent: string;
}

interface ProviderRegistration extends ProviderIdentity {
  id: ProviderId;
  automaticRoot: string;
  displayRoot: string;
}
```

The exact current Claude and Codex values remain in the registry. Branding
components read registration data; they do not branch on provider names.
Provider accent and logo treatment are generic metadata-driven variants. CSS
does not contain `.provider--claude`, `.provider--codex`, or a selector that
names a provider. A monochrome logo rule may select a generic
`data-logo-variant` value.

The registry remains a built-in list. The frontend may render only values
whose IDs pass `isProviderId`; test-only composition fixtures must not extend
the registry or weaken payload validators.

## Widget view model and composition

`src/lib/widget-view-model.ts` is pure. It consumes only existing typed
summary/settings values and an optional preview source-enabled map:

```ts
type WidgetColorMode = "dark" | "light";

interface WidgetViewModelInput {
  summary: UsageSummary;
  settings: WidgetSettingsSnapshot;
  previewSourceEnabled: Readonly<Record<ProviderId, boolean>> | null;
}

interface WidgetProviderViewModel {
  provider: ProviderId;
  identity: ProviderIdentity;
  status: { state: UsageState; label: string };
  metrics: {
    sessionTokens?: number;
    todayTokens: number;
    updatedLabel: string;
  };
}

interface WidgetViewModel {
  theme: ThemeId;
  colorMode: WidgetColorMode;
  providers: WidgetProviderViewModel[];
  totalTokens: number;
  visibleProviderCount: number;
}
```

The mapper iterates the canonical registry, filters by existing visibility,
marks a preview-disabled source unavailable for display, and recomputes the
existing total from enabled preview values. It never creates a `Session`
object, session ID, or provider payload. `sessionTokens` is the existing
aggregate field already present in `ProviderUsageSummary`.

`ProviderSection` owns the provider identity/status shell and accepts
`children: ReactNode`:

```ts
interface ProviderSectionProps {
  identity: ProviderIdentity;
  status: { state: UsageState; label: string };
  children: ReactNode;
}
```

`UsageMetrics` accepts a readonly list of display metrics and the existing
relative update label:

```ts
interface UsageMetric {
  label: string;
  value: string;
  ariaLabel: string;
}

interface UsageMetricsProps {
  metrics: readonly UsageMetric[];
  updatedLabel: string;
}
```

The current `ProviderUsageRow` is a small composition adapter from
`WidgetProviderViewModel` to these primitives. Future individual-session rows
can become additional children without changing the provider identity/status
shell or the Rust/React wire contract.

## Settings activity and persistence boundary

`useSettingsController` owns only editable settings state, source loading,
and user actions. It no longer subscribes to usage summaries. A separate
`useSettingsActivity` subscription produces the existing `UsageSummary` and
provider status views for `SettingsActivityPanel`; the same summary supplies
source health to the source section. The panel calls the hook internally and
renders both provider visibility and source settings; the screen and edit
controller never receive the live usage summary. The panel does not own edit state.

Lane C owns the pure settings-model relocation and leaves the component-path
compatibility re-export for B. C also replaces only the Tauri unlisten type imports
in `useUsageSummary` and `useWidgetSettings`; those hooks retain their behavior.
Current source signatures override illustrative interface sketches in this document.

The persistence hook has this stable shape:

```ts
interface UseSettingsPersistenceResult {
  sendPreview(preview: WidgetSettingsPreview): void;
  saveWidget(snapshot: WidgetSettingsSnapshot): void;
  saveSource(settings: SourceSettings): void;
  flush(): Promise<void>;
}
```

It owns the existing pending preview promise, serialized persistence queue,
source operation ordering, inline error reporting, and close flush. It keeps
the current queue ordering and does not add a new debounce or coalescing
policy. Unmount cleanup ignores new work while allowing an in-flight native
operation to settle. `closeSettings` awaits `flush()` before calling the
existing `closeCurrentWindow()` bridge. It never emits an old snapshot to roll
back an already previewed edit.

The hook does not introduce a state library, a global store, or a second
settings wire format. The native folder picker continues to persist through
the existing `pick_source_root` command and returns a validated snapshot.

## Desktop and contract boundary

Only modules under `src/lib/desktop/` may import `@tauri-apps/api` after this
slice. The intended split is:

- `desktop/commands.ts`: the exact existing `invoke` calls for
  `get_usage_summary`, `get_widget_settings`, `update_widget_settings`,
  `get_source_settings`, `pick_source_root`, and `update_source_settings`, as
  verified by the current facade tests. No new command name is introduced.
- `desktop/events.ts`: the exact existing `listen`/`emit` calls for
  `usage-summary-changed`, `widget-settings-changed`, and
  `widget-settings-preview-changed`.
- `desktop/window.ts`: `getCurrentWindow`, native drag/resize/close, and the
  existing logical sizing transport, with no new platform behavior.
- `desktop/platform-copy.ts`: existing Windows/approved-WSL source-root and
  native-window error copy. It contains no Tauri import.

Pure DTO definitions, strict unknown/unsafe payload validators, and formatters
live in `src/lib/contracts/`. The existing paths remain compatibility facades:

- `src/lib/usage-summary.ts`
- `src/lib/widget-settings.ts`
- `src/lib/widget-settings-preview.ts`
- `src/lib/source-settings.ts`
- `src/lib/window-actions.ts`
- `src/lib/window-sizing.ts`

Those facades re-export the pure types/parsers/formatters and delegate to the
desktop boundary with the same public function names, return types, error
codes, command names, event names, and validator rules. Existing hooks and
components continue importing these paths during the parallel worker phase.
No component imports a Tauri package directly.

## CSS Modules and semantic tokens

All component styles move under these responsibility folders:

```text
src/styles/globals/reset.css
src/styles/globals/tokens.css
src/styles/globals/themes.css
src/styles/widget/*.module.css
src/styles/settings/*.module.css
src/styles/shared/*.module.css
```

The old combined `index.css` entry and global component styles are removed or
left unused only after each window has its explicit imports. `main.tsx` imports
reset, tokens, themes, widget modules, and shared modules. `settings-main.tsx`
imports reset, tokens, themes, settings modules, and shared modules. The widget
bundle must not import settings modules, and the settings bundle must not
import widget modules.

Globals contain only the universal reset/base rules, semantic token slots, and
theme declarations. Component modules contain local selectors only. A parent
module never pierces a child module with descendant selectors or `:global`.

The semantic token set must cover every value currently consumed by the
runtime, including:

- Claude light/dark canvas, card, ink, body, muted, muted-soft, line, and
  positive/accent colors;
- the current `StyreneB`/Inter/system UI, Copernicus/Tiempos/Garamond system
  display, and monospace stacks;
- 4px-based spacing, current 8/12/16px radii, provider mark size, and focus
  ring;
- current widget rhythm, window padding, settings scrollbar gutter, and
  settings/widget elevation contracts; and
- current type roles, responsive size constants, and interaction timing.

Theme declarations are scoped to the root attributes:

```tsx
<main
  className={styles.root}
  data-theme={theme}
  data-color-mode={darkMode ? "dark" : "light"}
>
```

Modules consume semantic variables without `var(--claude-...)` fallbacks.
The only current theme selector is `[data-theme="claude"]`; the only current
color modes are `light` and `dark`. Existing class names may remain during the
TSX worker phase to keep the migration reviewable, but the final CSS has no
provider-specific branches and no shared global component selectors.

## Worker ownership and ordering

The following ownership is exclusive during each phase. A phase handoff is
required before another worker edits a transferred file.

| Lane | Files and responsibility | Depends on |
| --- | --- | --- |
| Task 0 | `src/tests/styles/panel-surfaces.test.mjs`, `src/tests/styles/widget-rhythm.test.mjs`; remove only stale preview reads and preserve runtime assertions | Recovery checkpoint |
| A | `src/lib/provider.ts`, `src/lib/widget-view-model.ts`; shared branding; `ProviderSection`, `UsageMetrics`, widget composition; widget/view-model/branding tests | Stable existing facades; Task 0 may run in parallel |
| B | `useSettingsActivity.ts`, `SettingsActivityPanel.tsx`, `useSettingsPersistence.ts`, `useSettingsController.ts`, settings TSX composition; split settings tests | Stable existing facades; Task 0 may run in parallel |
| C | `src/lib/contracts/*`, `src/lib/desktop/*`, existing `src/lib/*.ts` facades, pure settings-model relocation and compatibility export, usage/settings hook type imports, contract/boundary tests | Existing provider registry; Task 0 may run in parallel |
| D1 | Global reset/tokens/themes, widget/shared CSS Modules, widget/shared style imports, `main.tsx`, widget/shared style tests | A, B, C integrated and reviewed |
| D2 | Settings CSS Modules, settings style imports, `settings-main.tsx`, settings style/browser tests | D1 token/theme contract and B integrated |

Root owns integration, review checkpoints, staging, commits, and promotion of
the working documents to the canonical `docs/superpowers/` paths. Workers do
not stage, commit, push, create worktrees, or spawn helpers.

## Verification contract

Task-level verification is narrow and cumulative:

- Contract tests reject unknown/unsafe payload fields, invalid provider IDs,
  duplicate/missing provider entries, invalid themes, invalid token counts,
  and invalid dates while preserving current accepted payloads.
- Widget view-model and composition tests cover loading, active, idle,
  unavailable, stale, current totals, preview-disabled totals, canonical
  order, a fixture-only third provider section, and multiple metric children.
- Settings tests cover activity states, visibility/source/theme independence,
  immediate preview, serialized writes, write errors, close flush ordering,
  unmount cleanup, and native picker behavior.
- Desktop boundary tests assert exact command/event names and stable facade
  exports. A source-level import test proves pure contracts have no Tauri
  imports and components have no direct Tauri imports.
- CSS tests inspect actual module output/source contracts for semantic tokens,
  no Claude fallbacks in component CSS, no provider-named selectors, root
  `data-theme`/`data-color-mode`, and per-window import isolation.
- `npm run build` proves both Vite entries bundle. A packaged Windows smoke
  check verifies computed runtime styles and native behavior for drag, resize,
  picker, responsive widget height, theme/color-mode roots, and settings
  scroll/elevation. Jsdom class existence alone is not visual evidence.

At the final gate, run:

```text
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug
git diff --check
```

The final report separates the repaired baseline, frontend tests/build,
desktop/package checks, and Windows smoke evidence. It does not claim the
static preview exists or claim Linux/macOS support.
