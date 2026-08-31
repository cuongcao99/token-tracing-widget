# Overlay Presentation Slice — Design Update

**Date:** 2026-08-30
**Status:** Approved visual direction; implementation pending plan approval
**Decision owner:** User-selected direction C — Dark Focus Tile

## Related artifacts

- [Approved v1 design](2026-08-29-token-tracing-widget-design.md)
- [Apple visual reference](../../../DESIGN.md)
- [Approved comp](../../../.impeccable/mocks/overlay-dark-focus-tile.png)
- [Measured comp spec](../../../.impeccable/build/spec.json)

## Decision

The main overlay will use the Dark Focus Tile direction selected by the user.
It is a compact, dark, Apple-inspired utility surface with a single strong
numeric focal point, quiet metadata, hairline separators, and one blue status
accent. The design language is adapted from `DESIGN.md`; it does not imitate
Apple product branding, logos, marketing navigation, or photography.

This update records two explicit departures from the 2026-08-29 baseline:

1. The main overlay is **normal non-topmost by default**. Other windows may
   cover it.
2. The overlay is redesigned as a dark focus tile. The data contract,
   privacy boundary, provider behavior, and v1 content scope are unchanged.

## Feedback refinement — 2026-08-30

Native review showed that the compact surface was visually cramped and that an
idle-but-known session was presented as unavailable. The following refinement
is approved for the same overlay slice:

- When the newest valid event is older than two minutes, the state remains
  `Idle`, but the latest provider and current-session total remain visible.
  Only a source with no known valid event may omit those fields.
- The overlay canvas is adjusted to approximately 352 × 140 logical pixels.
  It remains frameless, transparent, non-resizable, taskbar-hidden, and
  non-topmost; the extra space is reserved for breathing room in the same
  header → hero → footer topology.

## Product and privacy invariants

- The product remains Windows 11-only, local-only, and metadata-only.
- Rust remains the only owner of filesystem, collection, SQLite, and source
  root access.
- React receives only the typed `UsageSummary`; it never receives raw session
  records, prompts, responses, reasoning, tool payloads, credentials,
  repository contents, or working directories.
- The overlay continues to show only provider state, current-session total,
  Today's Total, and relative last-update time.
- No chart, history view, model name, cost, token breakdown, network client,
  telemetry, sidecar, CSS framework, frontend state library, ORM, or new
  setting is introduced by this slice.

## Main overlay visual contract

Viewport: approximately 320 × 120 logical pixels, frameless, transparent,
taskbar-hidden, with the existing remembered-position behavior.

### Surface and tokens

Use the token family from `DESIGN.md`, adapted to the small desktop surface:

- Surface: `#272729` (near-black tile)
- Primary text: `#ffffff`
- Muted text: `#cccccc` / `#a1a1a6` where lower emphasis is needed
- Action/status blue on dark: `#2997ff`
- Hairline: a low-contrast one-pixel rule derived from the documented hairline
  token; no visible heavy border
- Radius: the compact utility radius from the design scale, approximately
  8–11px at the widget edge
- Shadow: none; the product-image-only shadow from `DESIGN.md` does not apply
  because this surface contains no imagery
- Gradient: none

Use system-resolved typography: `SF Pro Display`, `SF Pro Text`,
`system-ui`, `-apple-system`, `BlinkMacSystemFont`, and `Segoe UI` fallbacks.
Do not add a font package or network-loaded font. Display numbers use tight
tracking; labels and metadata use readable system text.

### Layout topology

1. Header: provider name at left; status mark and state at right. The whole
   header remains the drag region.
2. Hero: `Current session` label with the current-session number as the
   dominant visual element.
3. Footer: `Today` and today's number at left; relative update text at right.
4. Hairline rules separate header, hero, and footer. They are structure, not
   decoration.

The visible numeric values omit the repeated `tokens` suffix to protect the
   compact hierarchy. Their semantic accessible names must include the unit,
   for example `Current session: 1,234 tokens` and `Today: 5,678 tokens`.
This is a presentation-only change; `UsageSummary` remains unchanged.

State mapping remains data-driven:

- `active`: blue status mark and `Active`
- `idle`: muted status mark and `Idle`
- `loading`: muted status mark and `Loading`
- `stale`: muted status mark and `Stale`
- `unavailable`: muted/error-safe status treatment and `Unavailable`; the
  state must not rely on color alone

Loading, unavailable, missing current-session, and very large totals must fit
without horizontal overflow. The provider fallback remains `Token Tracing`.

## Settings surface contract

The settings window remains a decorated, resizable, non-topmost 520 × 560
window created on demand. It adopts the same Apple-inspired token family but
uses a light parchment workspace for the longer form:

- Page canvas: `#f5f5f7`
- Functional cards/fields: white or pearl surfaces with thin hairlines
- Text: near-black ink and muted gray hierarchy
- Primary action/focus: Action Blue and Focus Blue
- No card shadows, decorative gradients, emoji icons, or new provider/source
  behavior
- Existing source toggles, root inputs, loading/saving/error states, and
  privacy-safe success message remain intact
- Provider cards and form controls must remain usable at the existing window
  size and at narrower WebView widths without clipping

This slice does not add the deferred launch-on-login, opacity, reset-position,
clear-index/rebuild, or arbitrary file-picker features. The unimplemented
`Always on top` preference remains deferred; the main window default is
explicitly non-topmost until a separate preference design is approved.

## Window and interaction semantics

- Set the main Tauri window to `alwaysOnTop: false`.
- Keep the window frameless, transparent, non-resizable, and hidden from the
  taskbar.
- Keep the header as the only drag region. The metric body and footer are not
  drag regions.
- The overlay is a normal interactive window and is not click-through. It may
  receive focus and must not block clicks on other windows once another window
  is in front of it.
- A close request (including the normal Windows close gesture for the
  frameless window) hides the main window instead of terminating collection.
- Tray `Show`, `Hide`, `Settings`, and `Quit` IDs and behavior remain
  unchanged. Opening Settings continues to show and focus one decorated
  Settings window.
- Summary updates must not steal focus or move the window.
- No new overlay buttons are introduced solely for decoration. Keyboard
  focus requirements apply to Settings controls; the non-interactive overlay
  header remains a labeled drag region.

## Accessibility and motion

- Keep semantic headings, labels, and output relationships in the React
  markup; do not encode meaning only through position or color.
- Announce state/error changes politely without placing raw provider data in
  an accessibility payload.
- Provide visible `:focus-visible` treatment for Settings inputs, toggles, and
  the Save action using the documented Focus Blue.
- Preserve a usable keyboard order and readable contrast on both dark overlay
  and light settings surfaces.
- Use no decorative animation. If a future state transition needs motion, it
  must be short, non-essential, and disabled under `prefers-reduced-motion`.

## Design acceptance checks

Before implementation is considered ready for review:

1. A 320 × 120 screenshot reads as one dark utility tile with a clear session
   value, not as a dashboard or marketing card.
2. No shadow, gradient, emoji, Apple logo, extra control, or unrelated copy
   appears.
3. Active/idle/unavailable/stale/loading states remain legible without color
   being the only signal.
4. Large totals and unavailable values do not clip or overflow at normal and
   high-DPI Windows scaling.
5. Settings remains readable, keyboard-usable, and visually consistent
   without changing its Rust command contract.
6. The Windows smoke pass confirms the non-topmost, non-click-through,
   draggable-header, close-to-hide, tray, and taskbar semantics.
