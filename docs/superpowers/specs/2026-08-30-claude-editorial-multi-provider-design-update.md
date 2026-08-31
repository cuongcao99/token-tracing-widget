# Claude Editorial Multi-Provider Overlay Design Update

**Date:** 2026-08-30
**Status:** Approved for implementation from the reviewed variant D preview
**Scope:** Overlay presentation, widget preferences, per-provider summary data, and Settings frontend structure

## Decision

The reviewed variant D preview is the implementation authority for the next
slice. It uses a warm Claude-inspired editorial treatment for Token Tracing:
cream workspace surfaces, Copernicus/Tiempos-style serif display type,
StyreneB/Inter-style UI type, coral controls, warm dark product surfaces, and
hairlines instead of heavy chrome. This is a visual adaptation for Token
Tracing; it does not copy Anthropic's radial-spike mark or any Apple logo.

The preview's information architecture is also approved for the live product:
the overlay shows both supported Providers independently, with each Provider's
current-session total, Today's Total, activity state, and last update, followed
by one combined Total. Settings controls which Providers are visible in the
overlay, which Sources are collected, and whether the shared surface uses Dark
mode.

This update supersedes only the presentation and summary-shape portions of
`docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md` and
`docs/superpowers/specs/2026-08-30-overlay-apple-redesign-design-update.md`.
Windows 11, local-only, metadata-only, Rust-owned filesystem/SQLite access,
and existing tray/window lifecycle requirements remain unchanged.

## Live overlay contract

The overlay remains interactive, frameless, transparent, taskbar-hidden, and
non-topmost. Its compact tile is resized to approximately 440 × 300 logical
pixels so two Provider rows can remain readable at normal Windows scaling.
Only the header is a drag region. The overlay has no logo mark, no global
`Today` label above the Provider list, and no decorative controls.

The visual hierarchy is:

1. `Token Tracing` and the aggregate activity status in the header.
2. One row for each visible Provider, with Provider name/state, `Session`,
   `Today`, numeric values, and a relative update label.
3. `Total` with the sum of all enabled Providers' current-day values.

The Provider rows are fixed to Claude Code and Codex. A Provider may remain
visible while idle and retains its last known normalized totals. A disabled
Source has no newly collected data and is represented as unavailable rather
than causing the other Provider to disappear.

## Settings contract

The Settings window keeps the existing decorated, resizable, non-topmost
window and source-root editing behavior, but adopts the reviewed single-panel
layout:

- a `Settings` title with the short caption `Choose what stays visible.`;
- `Visible providers` with independent Claude Code/Codex switches;
- `Sources` with independent collection switches, health state, a compact
  configured-root label, and a `Change…` disclosure for the existing root
  input;
- `Appearance` with one `Dark mode` switch;
- one `Save changes` action and privacy-safe loading/error/success states.

The setting surface uses the same theme state as the overlay. Dark mode is
enabled by default to preserve the current widget appearance. Toggling the
switch changes the Settings surface immediately; saving persists it and
publishes the new state so the overlay and any open Settings view converge.

## Typed data contracts

Extend the existing privacy-safe `UsageSummary` with a fixed-order
`providers` collection. Each entry contains only:

```text
provider: "claude" | "codex"
state: "loading" | "active" | "idle" | "unavailable" | "stale"
currentSessionTokens?: non-negative safe integer
todayTokens: non-negative safe integer
lastUpdatedAt?: valid timestamp string
```

The existing aggregate `state`, active `provider`, active
`currentSessionTokens`, aggregate `todayTokens`, `lastUpdatedAt`, and
`sourceHealth` fields remain available for compatibility and status semantics.
`todayTokens` is the sum of the enabled Provider entries.

Add a separate typed widget-preferences contract:

```text
visibleProviders: [
  { provider: "claude" | "codex", visible: boolean },
  { provider: "claude" | "codex", visible: boolean }
]
darkMode: boolean
```

The preference command boundary is `get_widget_settings` and
`update_widget_settings`; successful updates emit the path-free
`widget-settings-changed` event. Defaults are both Providers visible and Dark
mode enabled. The existing `get_source_settings` and
`update_source_settings` contracts remain intact.

## Persistence and privacy

Widget preferences use the existing SQLite key/value `settings` table; no
schema migration is required. Use stable keys under `widget.*` for Provider
visibility and Dark mode. Invalid or missing preference values use the safe
defaults and never enter diagnostics.

Per-Provider totals are derived only from normalized `UsageEvent` rows. No
prompt, response, reasoning, tool payload, credential, repository content,
working directory, raw record, absolute source root, or arbitrary frontend
payload crosses the summary/event boundary. Source roots may appear only in
the Settings source-editing flow, as already allowed by the base design.

## Acceptance criteria

1. The production overlay visually matches the reviewed variant D structure:
   both Provider rows, `Session`/`Today`, `Total`, no `T` mark, and no global
   `Today` label.
2. Claude and Codex totals are independently derived from normalized events;
   the aggregate remains the sum of enabled Providers.
3. Visible-provider switches affect only overlay presentation; Source switches
   affect collection as before.
4. Dark mode persists in SQLite, updates both open surfaces through a typed
   event, and defaults to enabled.
5. Existing source-root validation, watcher refresh, tray behavior, close-to-
   hide behavior, and privacy boundaries remain intact.
6. Frontend source files are separated by responsibility into components,
   hooks, bridge modules, styles, and tests; no root component or stylesheet
   contains the whole UI.
7. Frontend tests, Rust tests, TypeScript/Vite build, integrated Tauri build,
   and the existing privacy checks pass.

## Explicitly out of scope

- network access, telemetry, accounts, cloud sync, or a frontend state library;
- model/cost/token-type breakdowns, charts, history views, or arbitrary files;
- Apple/Anthropic logos or provider-specific raw session content;
- launch-on-login, opacity, reset position, clear-index/rebuild, or Always on
  top preferences;
- changing provider adapters, collection-core normalization invariants,
  checkpoint semantics, or the existing source-root validation rules.

## Design update — 2026-08-31

Dark mode now has a reversible live-preview interaction. Toggling the switch
immediately previews the theme across the open Settings and overlay surfaces
through a typed, transient event; it does not write SQLite or replace the
persisted preference. `Save changes` remains the only persistence action.
Closing Settings before saving sends the persisted dark-mode value back as a
final preview and then closes the window, so an abandoned edit cannot leave a
temporary theme behind. The Settings close action has an explicit
`core:window:allow-close` capability and surfaces close failures instead of
silently ignoring them.

All three editable Settings switches now share the same reversible preview
boundary. Visibility and source-enabled changes update the open widget
transiently; a source-disabled Provider keeps its last known numbers but is
shown as unavailable, and the aggregate Total excludes it until the draft is
saved. Closing before Save restores the complete persisted snapshot. The
production widget remains 440px wide but grows to 300px high for clearer
vertical breathing room. Its title uses the coral display face, and a single
hairline separates the Provider list from Total.

## Design update — 2026-08-31 — responsive and resizable windows

The approved Claude-editorial surfaces now support bounded native resizing
without changing the local-only data boundary. The widget keeps its default
440px logical width, preserves a user-adjusted width when its height is
automatically synchronized, and uses these logical height targets based only
on visible Providers:

- 0 visible Providers: 176px
- 1 visible Provider: 228px
- 2 visible Providers: 300px

The widget's native resize range is 360–720px wide and 176–520px high. The
Settings window is also resizable within 440–820px wide and 420–900px high.
Both frameless surfaces expose eight transparent edge/corner resize hit areas
backed by Tauri's native resize API. Their draggable headers include a small
six-dot grip as a visual affordance; the grip is decorative and does not
replace the header drag region.

The widget type scale is now shared by the shipped webview and design preview:
the `Token Tracing` title is 19px and the aggregate `Total` number is 20px.
Provider names, metric labels, metric values, display font roles, and
tabular-number treatment remain unchanged. Window geometry is intentionally
not persisted in version one.
