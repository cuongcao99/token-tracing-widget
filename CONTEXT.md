# Token Tracing

Token Tracing is the domain of deriving privacy-safe token-usage totals from local coding-agent session data and presenting a current aggregate without retaining conversational content.

## Language

**Provider**:
A supported coding-agent product that produces local session data, currently Claude Code or Codex, with one canonical registered identity.
_Avoid_: Agent, integration

**Source**:
The user-enabled local session data belonging to one Provider. A Source may be
collected from the automatic or custom Windows root and one optional explicit
WSL root at the same time.
_Avoid_: Installation, feed

**Source Root**:
One configured Windows or WSL boundary within which a Source may be discovered
and read.
_Avoid_: Scan path, home directory

**Session**:
One opaque Provider-defined span of related coding-agent activity.
_Avoid_: Conversation, chat

**Session Identity**:
The stable opaque identity used to group Usage Events belonging to one Session.
_Avoid_: Session name, conversation ID

**Active Session**:
A Session whose newest valid Usage Event is inside the 15-second activity window.
_Avoid_: Current conversation, selected session

**Observation**:
Privacy-safe token metadata derived from one Provider record.
_Avoid_: Message, raw record

**Incremental Observation**:
An Observation whose token values represent new usage by themselves.
_Avoid_: Event total

**Cumulative Observation**:
An Observation whose token values represent usage accumulated up to that point.
_Avoid_: Delta

**Usage Event**:
A deduplicated token delta accepted into Token Tracing totals.
_Avoid_: Observation, raw event

**Monotonic Segment**:
A consecutive span in which a cumulative token counter does not decrease.
_Avoid_: Session, reset

**Checkpoint**:
A restart-safe collection position describing how much of a Source has already been processed.
_Avoid_: Bookmark, cursor

**Active Provider**:
The Provider with the newest valid Usage Event inside the activity window.
_Avoid_: Current agent, selected provider

**Current-session Total**:
The current-local-day sum for all Active Sessions of the Active Provider, retaining the most recently updated Session's current-day total while idle.
_Avoid_: Current tokens, conversation total

**Today's Total**:
The sum of accepted Usage Events within the current Windows local calendar day across enabled Providers.
_Avoid_: Daily usage, last 24 hours

**Source Health**:
The current ability to collect a Provider's configured Source roots independently of other Providers.
_Avoid_: App status, connection status

**Usage Summary**:
The privacy-safe aggregate presented to the overlay: activity state, optional Provider, current-session total, Today's Total, last update, and Source Health.
_Avoid_: Dashboard data, raw usage

**Provider Registry**:
The canonical set of supported Provider identities and their safe display/adapter metadata.
_Avoid_: Plugin marketplace, arbitrary provider

**Theme**:
A named visual token system applied to the widget and settings surfaces; Claude is the current theme.
_Avoid_: Dark mode, skin

## Repository shape

The implementation is a Windows-first Tauri 2 desktop app. Rust owns local
data access and the collection pipeline; React owns presentation and typed
Tauri bridge calls; plain CSS owns the visual system.

### Rust runtime and data flow

- `src-tauri/src/app/` contains startup, live collection orchestration, window
  lifecycle, and tray behavior.
- `src-tauri/src/providers/claude/` and `src-tauri/src/providers/codex/` keep
  provider-specific readers and parsers behind the shared adapter contract.
- `src-tauri/src/sources/` handles bounded source-root discovery, source
  configuration for Windows plus optional WSL roots, session-file enumeration,
  and file watching.
- `src-tauri/src/collection/` and `src-tauri/src/usage/` validate observations,
  convert cumulative counters to deltas, filter duplicates, calculate provider
  summaries, and select the Active Provider.
- `src-tauri/src/database/` persists normalized usage events, sessions,
  checkpoints, source configuration, and widget settings in SQLite.
- `src-tauri/src/commands/` exposes typed usage-summary, source-settings, and
  widget-settings commands. `src-tauri/src/types/` defines the contracts
  crossing the Rust/React boundary.

The privacy-preserving flow is:

`bounded Source Root → provider reader/parser → Observation → validated Usage Event → SQLite aggregates → typed Usage Summary → React surface`

Raw provider records and conversational content stay inside the Rust boundary.

### Frontend surfaces

- `index.html` and `src/main.tsx` host the widget; `settings.html` and
  `src/settings-main.tsx` host the settings window.
- `src/components/widget/` contains the widget header, provider usage rows,
  total, and composition.
- `src/components/settings/` contains provider visibility, source settings,
  appearance, close control, switches, and the settings composition. Pure
  snapshot transforms live in `settings-model.ts`; async orchestration lives
  in `src/hooks/useSettingsController.ts`.
- `src/components/shared/` contains the provider dot, six-dot WindowGrip, and
  native WindowResizeHandles shared by both windows.
- `src/hooks/` owns usage-summary and widget-settings subscriptions. `src/lib/`
  owns typed bridge validation, settings preview events, provider/source
  transforms, layout sizing, and window actions. `src/styles/` contains base,
  token, widget, settings, window-control, and shared layout styles.
- `src/tests/` mirrors the frontend responsibility folders. Rust integration
  and contract tests remain under `src-tauri/tests/`.

The former static `design-preview.html` surface and its preview-only sources
were retired during frontend modularity work. Runtime UI changes are reviewed
through the React/Tauri surfaces and must still be implemented in the React
components and their production styles.

## Current product state

The current implementation supports the registered Claude Code and Codex
Providers independently while keeping both visible when configured, including
when a Provider is idle. Multiple sessions for one Provider can contribute to
its Active current-session total. The widget presents each visible Provider's
current-session and Today's totals plus one combined `Total`; it does not
expose raw source data. Enabled provider roots are observed continuously while
the app is open, including one optional explicit WSL root per Provider; activity
is derived only from the newest valid token event and expires after 15 seconds
without a newer event.

Settings currently control:

- per-Provider widget visibility;
- per-Provider Source collection enabled state, automatic/custom Windows Root,
  and optional explicit WSL Root; and
- the shared Claude Theme and dark-mode preference.

Settings edits are previewed immediately to the widget through typed preview
events and auto-saved through the typed settings commands. Provider visibility,
source collection, and both platform-specific source roots persist immediately.
Closing waits for pending preview and persistence work and does not restore an
older snapshot. The settings
window has a close control, native drag support, native resize handles, a fixed
header, and a separately scrolling content body with a stable scrollbar gutter.
The widget and settings window share the six-dot drag affordance, native resize
handles, responsive layout, and frameless transparent non-topmost taskbar-
hidden shell behavior. Window geometry is intentionally not persisted.

The settings scrollbar extends into the shell's right padding while preserving
the content inset, so the thumb stays at the panel edge without changing card
width when overflow appears. WebKit scrollbar arrow buttons are hidden. Both
surfaces are borderless; settings uses diffuse negative-spread CSS elevation,
while the widget remains shadow-free to avoid a rectangular perimeter artifact.
The native Tauri shadow remains disabled and no crisp near-edge shadow layer is
used.

The widget keeps a breathable responsive height for zero, one, or two visible
Providers (target heights 192, 244, and 316 logical pixels), preserves a
manually resized width within 360–720 logical pixels, and clamps its height to
the visible-provider target through 520 logical pixels. Its `Token Tracing`
title uses the same 32px display-title role as Settings. Its six-dot grip is
the top-center native drag affordance; edge and corner handles remain native
resize controls. Settings uses 440–820 logical pixels for width and 420–900
logical pixels for height, with a 600px default. Transparent frameless windows
retain layered CSS elevation without a native perimeter shadow. These bounds
are implementation contracts, not a persistence format.

Current-session values are scoped to the current Windows local calendar day.
Historical events can preserve a Provider's state and last-update timestamp,
but a Provider with no event today exposes zero current-session tokens. Today's
aggregate still sums enabled-provider events for the injected local day.

The maintained visual references are:

- `design/DESIGN_APPLE.md`: the user-provided Apple visual analysis and token
  reference;
- `design/DESIGN_CLAUDE.md`: the Claude-editorial visual direction used for the
  current review and runtime presentation; and
- `PRODUCT.md`: product scope and brand commitments.

The dated specs under `docs/superpowers/specs/` record approved departures and
refinements; they remain authoritative for behavior changes. The current
frontend uses React 19, TypeScript, Vite, Vitest, and plain CSS. No frontend
state library, CSS framework, font package, network client, telemetry,
sidecar, background service, or ORM is part of the approved implementation.

## Verification snapshot

The latest implementation baseline was verified with:

- frontend: `npm test -- --run` — 139 tests passing across 34 test files;
- frontend build: `npm run build` — passing;
- Rust formatting: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` — passing;
- Rust compile: `cargo check --manifest-path src-tauri/Cargo.toml` — passing;
- Rust tests: `cargo test --manifest-path src-tauri/Cargo.toml` — 136 tests passing;
- debug package build: `npm run tauri build -- --debug` — passing, producing
  `src-tauri/target/debug/token-tracing-widget.exe` and the NSIS installer at
  `src-tauri/target/debug/bundle/nsis/Token Tracing Widget_0.1.0_x64-setup.exe`.

Packaged Windows manual smoke coverage for drag, resize, and multi-window
placement has not yet been completed, so automated success should not be read
as a substitute for that check.

## Known follow-ups

- Source settings and widget settings are persisted sequentially rather than
  through one atomic transaction.
- Auto-save retry feedback is inline; a future slice may add a more explicit
  persistence status history without changing the immediate-save contract.
- Historical Windows local-day calculations need a DST-focused follow-up.
- `.impeccable/` contains generated review artifacts and browser profiles. Only
  intentionally shared `config.json` files under that directory are eligible
  for commits; local config overrides and generated state remain ignored.
