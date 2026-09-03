# Token Tracing Widget Settings Auto-save and Surface Depth — 2026-08-31

This update records the approved change from explicit save/restore settings to
auto-save, and the related settings layout and surface-elevation refinements.
It supersedes only the settings persistence interaction, settings scroll
boundary, default settings height, and panel shadow details. The product,
privacy, provider, collection, and native-window boundaries remain unchanged.

## Settings persistence

- Provider visibility, source collection enabled state, and dark mode preview
  immediately in both the settings panel and widget, then persist automatically.
- Source-root text changes persist after a short debounce of approximately
  350ms and flush on input blur. No Save button or submit step is required.
- Persistence is serialized and coalesced so rapid edits cannot write an older
  snapshot after a newer edit.
- A persistence error remains visible in the settings panel while the current
  preview remains usable for retry through a later edit.
- Closing settings waits for pending preview/persistence work and does not
  restore an older saved snapshot, because edits are already persisted.

## Settings layout

- The settings root keeps a fixed panel dimension and owns no content scroll.
- The header remains visible while a dedicated content region scrolls below it.
- The content region reserves scrollbar space with `scrollbar-gutter: stable`,
  keeping the panel width and inner layout unchanged when the scrollbar appears.
- The default settings height increases from 560px to 600px. Existing native
  minimum and maximum resize bounds remain unchanged.

## Surface elevation

- Widget and settings surfaces remain borderless, transparent-window-safe,
  frameless, taskbar-hidden, and non-topmost.
- Native Tauri shadow remains disabled because it creates a perimeter artifact.
- CSS elevation becomes more pronounced through a crisp near-surface shadow and
  a larger soft ambient shadow, with separate light/dark alpha treatment. No
  white or transparent border is introduced.

## Typography

- `Token Tracing` uses the same 32px display-title role as `Settings`.
- Responsive widget target heights increase enough to preserve provider-row
  breathing room and prevent clipping after the larger title is applied.

## Verification contract

- Frontend tests cover auto-save calls, source-root debounce/blur flush, no Save
  button, close behavior, and settings scroll structure.
- Layout tests cover the title-safe responsive widget targets.
- Frontend and Rust checks remain required according to `AGENTS.md`; no Rust
  collection behavior changes are expected for this slice.
