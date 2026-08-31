# Session, Provider Registry, and Theme Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task with review checkpoints.

**Goal:** Extend the verified widget/settings presentation boundary so multiple
sessions per provider aggregate correctly, provider-specific behavior is
registry/adapter driven, and Appearance exposes a persisted Claude theme
selector without changing the current visual contract.

**Architecture:** Keep all identity, aggregation, provider adapters, source
configuration, persistence, and validation in Rust. Add a canonical Rust
provider order plus built-in adapter registry; mirror it with a typed frontend
provider registry. Add a `Theme`/`ThemeId` registry and pass the theme through
the existing typed settings snapshot/preview flow. Refactor the current-session
calculation into a pure session aggregation seam and keep the existing summary
shape safe and opaque.

**Tech Stack:** Rust, Tauri 2, SQLite via rusqlite, React 19, TypeScript,
Vitest, Vite, plain CSS, Cargo tests, and the existing Impeccable detector.

**Spec:** `docs/superpowers/specs/2026-08-31-session-provider-theme-refactor-design-update.md`

## Global Constraints

- Work only on `refactor/ui-ux`; do not merge or push in this task.
- Read and preserve `CONTEXT.md`, `PRODUCT.md`, `design/DESIGN_APPLE.md`,
  `design/DESIGN_CLAUDE.md`, the base design spec, and dated approved design
  updates already reviewed for this repository.
- Keep version one Windows 11-only, local-only, metadata-only, and free of
  network/telemetry/sidecar/state-library/CSS-framework/ORM/font additions.
- Do not expose session identities or raw provider/source data to React,
  SQLite diagnostics, or normalized frontend-facing contracts.
- Preserve Claude-editorial colors, type scale, spacing rhythm, six-dot grip,
  responsive/resizable sizing, shadow-free widget, settings elevation, native
  drag/resize behavior, and immediate preview/auto-save/close-flush semantics.
- Use `apply_patch` for source edits. Keep generated build/review artifacts
  ignored under `.impeccable/`.
- Follow TDD: each behavior change gets a failing focused test before its
  production implementation, then a minimal implementation and a passing
  rerun.

---

## Task 1: Record the approved design and domain vocabulary

**Files:**

- Add `docs/superpowers/specs/2026-08-31-session-provider-theme-refactor-design-update.md`.
- Add this plan at `docs/superpowers/plans/2026-08-31-session-provider-theme-refactor.md`.
- Update `CONTEXT.md` glossary and current-state wording for Session Identity,
  Active Session, Current-session Total, Provider Registry, and Theme.

**Steps:**

1. Confirm the handoff's strict visual contract and implementation authorization
   are reflected in the design update.
2. Document the 120-second multi-session aggregation rule, idle fallback,
   registry/adapter seam, theme compatibility, privacy boundary, and out of
   scope items.
3. Update only the domain definitions and current-state descriptions needed to
   keep the repository vocabulary accurate; do not add implementation details
   to the glossary.
4. Run `git diff --check` and inspect the diff for scope before committing the
   documentation checkpoint.

**Expected result:** The approved design and executable plan exist before
production code changes, and the glossary describes the resolved domain terms.

## Task 2: Add failing Rust tests for concurrent session aggregation

**Files:**

- `src-tauri/tests/provider_summary.rs`
- `src-tauri/tests/session_summary.rs`

**Steps:**

1. Add a test with two Claude `UsageEvent`s using different session keys, both
   within the 120-second window, asserting Active state and the summed
   current-local-day current-session total.
2. Add a test proving the idle fallback retains the most recently updated
   session's current-day total when no session is active.
3. Keep the existing current-day boundary and provider-independence tests.
4. Run the narrow commands:
   `cargo test --manifest-path src-tauri/Cargo.toml --test provider_summary aggregates_concurrent_active_sessions_for_active_provider`
   and
   `cargo test --manifest-path src-tauri/Cargo.toml --test provider_summary retains_latest_session_total_when_provider_is_idle`.

**Expected result:** The new focused tests fail for the current single-session
implementation, demonstrating the missing behavior before production edits.

## Task 3: Implement the Rust session aggregation seam

**Files:**

- Add `src-tauri/src/usage/session_summary.rs`.
- Update `src-tauri/src/usage/mod.rs`.
- Update `src-tauri/src/usage/active_provider.rs`.
- Update `src-tauri/src/usage/provider_summary.rs` to consume the session seam.
- Update `src-tauri/src/collection/mod.rs` to use canonical provider order for
  summary provider iteration.

**Steps:**

1. Implement a pure internal aggregation function that groups already-safe
   `UsageEvent`s by opaque `session_key`, selects each session's newest valid
   event, marks active sessions using the 120-second window, and sums their
   current-day totals.
2. Preserve timestamp validation, saturating token arithmetic, latest-event
   ordering, unavailable behavior, and the existing idle last-known fallback.
3. Make `compute_active_provider` select the newest Provider first, then use the
   session seam for that Provider; make the local-day helper explicitly retain
   the current-day zero behavior for historical-only events.
4. Replace hard-coded summary provider arrays with `Provider::all()` wherever
   this seam is touched.
5. Run the focused provider-summary/collection tests and then the full Rust
   unit/integration suite for this checkpoint.

**Expected result:** Concurrent active sessions aggregate correctly, existing
summary behavior remains green, and no session identity enters a wire type.

## Task 4: Add failing Rust registry and theme contract tests

**Files:**

- `src-tauri/tests/provider_registry.rs`.
- `src-tauri/tests/widget_settings_contract.rs`.
- `src-tauri/tests/widget_settings_persistence.rs`.
- `src-tauri/tests/database.rs` where persistence setup is already covered.
- Add focused registry tests under `src-tauri/tests/provider_registry.rs` if
  that gives the registry a direct contract.

**Steps:**

1. Test canonical provider ordering and lookup of Claude/Codex adapter
   registrations.
2. Extend widget-settings contract expectations with `theme: "claude"`.
3. Add a persistence test that saves a Claude theme snapshot, reloads it, and
   verifies the theme; also verify an existing settings database with no theme
   row defaults to Claude.
4. Add a command-input test proving an omitted theme defaults to Claude and
   duplicate/missing provider entries remain rejected.
5. Run the focused commands `cargo test --manifest-path
   src-tauri/Cargo.toml --test provider_registry` and `cargo test
   --manifest-path src-tauri/Cargo.toml --test widget_settings_contract` and
   verify these new expectations fail before the
   implementation is added.

**Expected result:** The Rust test suite captures the new registry and theme
contracts and is red only for the unimplemented behavior.

## Task 5: Implement Rust provider registry, generic provider loops, and theme persistence

**Files:**

- Add `src-tauri/src/providers/registry.rs`; update
  `src-tauri/src/providers/mod.rs` and `provider_adapter.rs` only as needed.
- Update `src-tauri/src/types/provider.rs`, add
  `src-tauri/src/types/theme.rs`, update `src-tauri/src/types/mod.rs`,
  `widget_settings.rs`, and `usage_summary.rs`.
- Update `src-tauri/src/database/settings.rs`.
- Update `src-tauri/src/commands/widget_settings.rs`.
- Update `src-tauri/src/sources/source_config.rs` and
  `src-tauri/src/sources/session_files.rs` only to replace duplicated provider
  iteration with the canonical order.
- Update `src-tauri/src/app/runtime.rs` to construct collection sources from
  the registry and watch the registered provider roots.
- Update `src-tauri/src/collection/mod.rs` summary provider iteration.

**Steps:**

1. Add `Provider::all()` as the canonical built-in order and a static registry
   mapping each provider to its existing adapter; do not change parser/privacy
   behavior.
2. Refactor settings/source/summary loops to consume canonical provider order;
   preserve serialized order and existing source-health semantics.
3. Add `Theme::Claude` with safe serde/display parsing and a default. Add the
   `theme` field to the snapshot, load/save `widget.theme`, and keep old rows
   defaulting to Claude.
4. Extend `WidgetSettingsInput` with a serde-defaulted theme, validate provider
   entries against canonical order, and construct snapshots in canonical order.
5. Build runtime `ProviderSource` values from registry registrations while
   preserving source-specific configured roots and invalid-settings diagnostics.
6. Rerun the focused tests from Tasks 3–4, then `cargo fmt --manifest-path
   src-tauri/Cargo.toml -- --check` and `cargo test --manifest-path
   src-tauri/Cargo.toml`.

**Expected result:** Rust has one provider registry/adapter seam, persisted
theme support is backward-compatible, and all Rust contracts pass.

## Task 6: Add failing frontend registry/theme/preview tests

**Files:**

- `src/tests/lib/widget-settings.test.ts`.
- `src/tests/lib/widget-settings-preview.test.ts`.
- Add `src/tests/lib/provider.test.ts` and `src/tests/lib/theme.test.ts` for
  direct registry/parser coverage.
- `src/tests/components/settings/SettingsScreen.test.tsx`.
- Update existing typed settings fixtures in affected frontend tests.

**Steps:**

1. Extend valid settings/preview fixtures and strict-key expectations with
   `theme: "claude"`.
2. Add tests that the provider registry preserves Claude/Codex order and that
   unknown theme IDs are rejected.
3. Add a SettingsScreen test for a Claude Theme select and its preview payload;
   assert auto-save includes the theme while dark mode and provider/source
   controls remain independent.
4. Run the focused Vitest files before changing implementation and record the
   expected failures.

**Expected result:** Frontend tests describe the new safe contract and UI
behavior before the implementation exists.

## Task 7: Implement frontend registries, typed theme flow, and Appearance selector

**Files:**

- Update `src/lib/provider.ts`; add `src/lib/theme.ts`.
- Update `src/lib/usage-summary.ts`, `widget-settings.ts`,
  `widget-settings-preview.ts`, and `settings-model.ts`.
- Update `src/hooks/useWidgetSettings.ts` and
  `src/hooks/useSettingsController.ts`.
- Update `src/components/settings/AppearanceSection.tsx`,
  `SettingsScreen.tsx`, `ProviderVisibilitySection.tsx`, and
  `SourceSettingsSection.tsx`.
- Update `src/components/widget/TokenTracingWidget.tsx` and
  `src/components/shared/ProviderDot.tsx`.

**Steps:**

1. Derive `ProviderId`, order, metadata, and `isProviderId` from one typed
   `providerRegistry`; make consumers iterate it instead of duplicated provider
   pairs.
2. Add `ThemeId`, `themeRegistry`, `themeOrder`, and `isThemeId` with Claude as
   the current option.
3. Parse/serialize theme in snapshot and preview bridges, default it in the
   settings hook, and carry it through controller preview, coalesced auto-save,
   persisted settings, and close flushing.
4. Add a compact native `<select>` labeled Theme to Appearance. Keep the
   existing Dark mode switch and all existing settings semantics.
5. Add theme classes and a generic registry-driven provider accent variable
   without changing current Claude token values or provider dot appearance.
6. Run focused frontend tests, then `npm test -- --run` and `npm run build`.

**Expected result:** The UI has a typed Claude theme selector, future-ready
registry seams, unchanged current presentation, and passing frontend tests.

## Task 8: Update the static review surface and run the UI detector

**Files:**

- `src/design-preview.js`.
- `src/design-preview.css` only for the selector/control styling needed to
  represent the runtime Appearance surface.
- `src/styles/base.css`, `src/styles/tokens.css`, `src/styles/widget.css`, and
  `src/styles/settings.css` only for the selector focus/font and semantic theme
  hooks/generic provider accent variables; preserve all existing values.

**Steps:**

1. Add the current Claude option to the static Appearance preview so the review
   surface reflects the runtime contract.
2. Run the Impeccable detector once after all UI edits:
   `node C:\Users\caocu\.codex\plugins\cache\impeccable\impeccable\4.1.2\skills\impeccable\scripts\detect.mjs --json src/components/settings/AppearanceSection.tsx src/components/settings/SettingsScreen.tsx src/components/settings/ProviderVisibilitySection.tsx src/components/settings/SourceSettingsSection.tsx src/components/widget/TokenTracingWidget.tsx src/components/shared/ProviderDot.tsx src/lib/theme.ts src/styles/base.css src/styles/tokens.css src/styles/widget.css src/styles/settings.css src/design-preview.css src/design-preview.js`.
3. Inspect findings against the approved Claude-editorial contract and fix only
   actionable regressions introduced by this task. Do not scaffold a separate
   hero/reference page or widen the visual redesign.

**Expected result:** Static and runtime presentation agree, detector evidence
   is captured, and no unrelated visual changes are introduced.

## Task 9: Refresh glossary and complete verification

**Files:**

- `CONTEXT.md` if any terminology or current-state statement still needs
  correction after implementation.
- No unrelated files.

**Steps:**

1. Run fresh final verification and inspect each exit code/output:

   - `npm test -- --run`
   - `npm run build`
   - `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
   - `cargo check --manifest-path src-tauri/Cargo.toml`
   - `cargo test --manifest-path src-tauri/Cargo.toml`
   - `npm run tauri build -- --debug`

2. Run `git diff --check` and inspect `git status --short` to ensure generated
   output and unrelated changes are absent.
3. Review the final diff for privacy, typed boundary, registry, multi-session,
   theme, auto-save, and strict visual-contract regressions.
4. Commit the focused implementation on `refactor/ui-ux`; do not push or merge.

**Expected result:** Verification is fresh and evidence-backed, the working
tree contains only the requested implementation/docs/tests, and the branch is
ready for user review with any unavailable manual Windows smoke check clearly
identified.
