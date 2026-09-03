# Token Tracing Widget Polish Design Update — 2026-08-31

This update records the approved refinements requested after the responsive,
resizable window implementation. It supersedes only the affected interaction,
window-boundary, spacing, and current-session presentation details; the
product, privacy, provider, and settings persistence contracts remain unchanged.

## Window surfaces

- The widget and Settings surfaces remain frameless, transparent,
  taskbar-hidden, non-topmost utility windows.
- Native Tauri window shadow is disabled because it produces a visible
  perimeter artifact around transparent frameless windows. The existing CSS
  elevation on the widget and Settings surfaces remains the product shadow.
- The panel roots must not render a visible white or transparent border line.

## Drag and resize interaction

- Each panel has one small six-dot grip, centered on the top border and given a
  subtle bordered hit target so its purpose is discoverable.
- The grip is the only window-move affordance. Native dragging starts from the
  grip itself; the title, provider rows, controls, and content are not drag
  regions.
- The grip is keyboard-focusable and exposes an accessible move label. Native
  edge and corner resize handles remain available separately.
- The widget's effective minimum height follows the visible-provider target:
  176px for zero providers, 228px for one, and 300px for two. Manual resizing
  cannot reduce the panel below the target needed to show its provider data.
- Provider rows keep deliberate vertical padding and are never clipped by an
  intermediate overflow container.

## Current-session day boundary

- A provider's `currentSessionTokens` is scoped to the current Windows local
  calendar day. Historical events may still provide the provider's state and
  last-update timestamp, but a session whose latest event is from yesterday
  renders as `0` for today's current-session value.
- Today's total remains the sum of accepted enabled-provider events whose local
  calendar day matches the injected clock day.
- No raw provider records, prompts, responses, credentials, or arbitrary file
  contents enter the frontend or normalized summary.
