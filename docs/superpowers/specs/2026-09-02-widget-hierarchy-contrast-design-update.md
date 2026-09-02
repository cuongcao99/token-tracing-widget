# Widget hierarchy and contrast refinement

Date: 2026-09-02  
Status: Approved in chat

## Intent

Refine the existing widget and Settings presentation without changing the
collection, summary, persistence, or privacy contracts. The refinement keeps
the warm Claude editorial world and makes the primary reading path clearer for
both glanceable and first-use interactions.

## Design authority

- `design/DESIGN_CLAUDE.md` remains the visual authority: warm canvas, coral
  brand action, serif display voice, humanist sans UI, restrained elevation,
  and semantic teal/amber state accents.
- Impeccable `polish`, `layout`, `clarify`, and `craft-floor` guidance supplies
  the review bar: preserve the incumbent world, use semantic tokens, make the
  task hierarchy obvious, keep copy actionable, and verify contrast/focus in
  the real surfaces.
- The approved multi-session contract remains unchanged. Session rows,
  aggregate metrics, disclosures, and total values retain their current data
  and behavior.

## Decisions

### Widget hierarchy

- Remaining quota values lead when rate limits are present.
- `Session` and `Today` remain available as supporting metrics with quieter
  type and contrast.
- The combined `Total` remains for product continuity, but becomes a quiet
  footer summary rather than a second hero metric.
- The activity phrase uses muted neutral text. Provider status keeps semantic
  state color: healthy active is positive, while idle/loading/stale remain
  neutral.
- Empty state text is readable, non-italic, and does not rely on reduced
  opacity.

### Settings semantics and density

- The two visually similar switch groups receive section hints: `Show in
  widget` and `Collect data from`.
- Source health uses green for ready, amber for limited/attention states, red
  for source errors, and neutral for disabled/unavailable/checking states.
- Settings rows and section gaps tighten by roughly ten percent while keeping
  existing controls, keyboard order, scroll behavior, and hit targets.
- Provider names and theme values use the humanist UI face. Serif remains for
  the Settings title and section headings.

### Accessibility

- Existing native `button`, `switch`, `listbox`, and disclosure semantics stay
  in place.
- Switches, theme picker, close button, source-root button, and disclosure
  retain visible `:focus-visible` rings sourced from the shared token system.
- Color never becomes the only source-health signal; the visible state label
  remains present.

## Scope

Change only the React presentation, semantic style tokens, focused frontend
tests, and this design record. Do not modify Rust, DTOs, provider readers,
SQLite, dependencies, or window/data boundaries.

## Follow-up: daily session refinement

- Active session rows use a small inline `Current` marker in the existing
  positive teal. It is text, not a filled badge, so the row stays editorial.
- The widget scrollbar uses a warm-neutral semantic thumb and a slightly wider
  gutter. Coral remains reserved for the brand/action accent.
- The redundant `Total` footer is hidden when exactly one provider is visible;
  the combined footer remains when multiple providers are visible.
- Session labels keep a flexible column with CSS ellipsis while token values
  stay in a max-content column. Existing opaque-ID fallback behavior is kept.
- Source health dots inherit the state label's `currentColor`, keeping Ready,
  Limited, and error states visually coherent.

## Follow-up: continuous quota color

- Quota fills and their percentage values use one continuously interpolated,
  Claude-toned HSL color per bar: hue progresses from `0` to `134`, saturation
  from `65%` to `39%`, and lightness from `50%` to `54%`. Red represents no
  remaining capacity and the full-capacity endpoint matches the
  `DESIGN_CLAUDE` success green (`#5db872`).
- This interpolation is limited to quota health. Source-health dots keep their
  discrete semantic colors, and coral remains reserved for Claude brand/action
  moments.

## Follow-up: vertical resize and activity pacing

- The widget keeps its content-fit target capped at 520 logical pixels, while
  the native minimum height remains the visible-provider baseline. Long session
  lists scroll inside the widget instead of collapsing the resize range.
- Non-active activity phrases rotate on an 8–12 second cadence so the muted
  editorial status line can be read before it changes. Active phrases retain
  their existing 15-second cadence.

## Follow-up: warm status color and activity fallback

- Healthy active/available states use the `DESIGN_CLAUDE` success green
  (`#5db872`); amber remains reserved for limited/quota warning states.
- The newest valid event keeps a provider Active for 15 seconds before the
  fallback state becomes Idle, including live expiry scheduling.

## Follow-up: session count placement

- Remove the separate provider-level `N sessions today` line.
- The `Session` supporting metric now shows the current-day session count;
  `Today` remains the provider token total, while current-session tokens stay
  visible on the active session rows.

## Follow-up: resize content anchor

- The widget keeps its native vertical bounds at the visible-provider baseline
  through 520 logical pixels. Its content-fit target ends 17px after the last
  rendered provider content.
- The anchor gap is applied to the window target rather than added as
  scrollable content, so it does not create a scrollbar. Scrolling begins only
  when real content exceeds the maximum height or the user resizes below the
  content anchor. Collapsed Idle anchors below its disclosure button; expanded
  Idle anchors below its last session row.
- Height synchronization is intentionally narrow: the initial render and Idle
  disclosure changes may re-fit the window, while provider visibility, source
  health, collection toggles, theme, token-only updates, and live data changes
  do not reset a manually resized height.
