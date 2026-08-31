# Session, Provider Registry, and Theme Refactor

**Date:** 2026-08-31
**Status:** Implementation-ready design update
**Scope:** Approved presentation-supporting domain refactor from the UI/UX handoff

## Context

The current runtime and presentation are verified on `dev` at `29051d2`. The
widget and settings surfaces have a deliberately compact Claude-editorial
visual system. The next slice must make the underlying interfaces extensible
without changing that visual contract: more than one active session may exist
for a provider, provider-specific behavior must be registered behind adapters,
and Appearance must expose a theme selector that is ready for future themes.

The user handoff authorizes implementation inline on branch `refactor/ui-ux`.
This document is the design gate for that implementation.

## Goals

- Aggregate multiple concurrently active sessions for the same Provider while
  keeping session identity opaque and out of the frontend/database diagnostics.
- Make the supported-provider set and provider adapter seam canonical so adding
  a built-in Provider does not require hard-coding widget composition in every
  consumer.
- Add a persisted `theme` preference and Appearance selector with Claude as the
  only current option, while retaining the existing `darkMode` preference and
  all current Claude-editorial visual values.
- Preserve the Rust ownership, metadata-only privacy boundary, typed frontend
  contracts, source-health independence, settings preview/auto-save behavior,
  and Windows utility-window behavior.

## Domain decisions

### Session identity and aggregation

`UsageEvent.session_key` is the stable opaque identity used to group events for
one provider session. It is derived from a provider session key when available,
and otherwise from the bounded source file's opaque identity. The key is an
internal grouping value; it is never serialized into `UsageSummary`, SQLite
diagnostics, or React payloads.

An Active Session is a session whose newest valid event is no more than 120
seconds old relative to the collection clock. For a selected Provider:

1. If one or more sessions are active, `current_session_tokens` is the sum of
   their accepted current-local-day events.
2. If no session is active but the Provider has history, the value is the
   current-local-day total of the most recently updated session. This preserves
   the existing useful idle display instead of dropping to zero.
3. Provider state is Active when any session is active, otherwise Idle when
   usable history/source exists. An empty and unusable Provider remains
   Unavailable.

The Active Provider remains the Provider with the newest valid event in the
activity window. Its current-session value is calculated using the aggregation
above, so concurrent sessions for that Provider are included without exposing
their identities. Existing day-boundary behavior remains: prior-day history
may make a Provider known/Idle, but its current-local-day total is zero.

### Provider registry and adapter seam

Rust owns a canonical `Provider::all()` order and a registry of built-in
`ProviderRegistration` entries. Each entry supplies a Provider identifier and
its `ProviderAdapter`; runtime collection constructs sources from this
registry. Summary, settings, source configuration, and persistence loops use
the canonical provider order rather than duplicated Claude/Codex pairs.

The frontend mirrors the same idea with a typed `providerRegistry` containing
display metadata, accent, and automatic-root label. Widget and settings
composition iterate that registry. Provider-specific parsing remains behind
the Rust adapter implementations. This is a built-in registry, not dynamic
plugin loading or arbitrary provider identifiers; adding a provider still
requires an enum/adapter/registry entry and its metadata.

### Theme contract

`Theme::Claude` / `ThemeId = "claude"` is the only current theme. The typed
widget-settings snapshot and preview now carry `theme` alongside the existing
`darkMode` boolean. Missing persisted theme values default to Claude, so
existing installations require no migration. The settings command accepts a
missing theme as Claude for compatibility, while frontend parsers remain strict
about the current safe payload shape.

Theme registration and class names are separated from semantic tokens so a
future theme can provide the same token slots without changing component
composition. The Claude token values, typography, spacing, border treatment,
widget shadow-free surface, settings diffuse elevation, responsive sizing,
native grip, and resize handles do not change in this slice.

## Boundary and privacy invariants

- Filesystem reads, source discovery, provider parsing, session grouping,
  normalization, delta conversion, deduplication, SQLite, and persistence stay
  in Rust.
- React receives only typed safe summaries/settings/preview data. No session
  key, prompt, response, reasoning, tool payload, credential, repository path,
  working directory, raw provider record, or arbitrary file content crosses
  the boundary.
- No network client, telemetry, sidecar, frontend state library, CSS framework,
  ORM, or font package is introduced.
- Existing auto-save, serialized/coalesced writes, immediate preview, close
  flushing, and independent provider source-health semantics remain intact.

## Focused acceptance tests

- Rust usage tests prove two same-provider sessions can be active at once and
  their current-day totals are summed; idle fallback and previous-day behavior
  remain covered.
- Rust registry/settings tests prove canonical provider iteration, adapter
  lookup, Claude defaulting for old settings rows, and persisted theme round
  trips.
- Rust command contract tests prove `theme` is serialized and missing input
  defaults to Claude while invalid provider sets remain rejected.
- Frontend bridge tests reject unsafe/unknown theme values, preserve registry
  order, and parse the new preview/snapshot shape.
- Settings UI tests prove the Claude theme selector previews and auto-saves
  without changing dark-mode or provider/source independence.
- Existing frontend, Rust, packaged debug build, privacy, and native-window
  contract tests remain green. The Impeccable detector is run once against the
  changed UI targets after implementation.

## Out of scope

- A second visual theme or a theme editor.
- Dynamic third-party provider plugins or arbitrary runtime provider IDs.
- A new wire field exposing active-session identities/counts.
- Reworking the approved Claude-editorial composition, layout proportions,
  typography, colors, or native window behavior.
