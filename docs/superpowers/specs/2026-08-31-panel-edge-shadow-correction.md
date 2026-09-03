# Panel edge and shadow correction — 2026-08-31

This design update corrects the approved panel presentation after Windows
smoke review. It refines only scrollbar placement and surface elevation; the
widget/settings behavior, native window contract, and local-only boundary stay
unchanged.

## Settings scrollbar

- The settings content region keeps its stable scrollbar gutter.
- Its scroll box extends through the panel's right padding so the scrollbar is
  visually aligned with the panel edge.
- The content keeps an equal right padding inset, so cards and controls do not
  move when the scrollbar appears.
- WebKit scrollbar buttons are hidden so no native arrow caps compete with the
  custom thumb.

## Borderless elevation

- The visible panel surface remains `border: 0` and `outline: none`.
- Native Tauri shadow remains disabled; no native-window setting changes are
  required.
- The crisp `0 1px 2px` CSS layer is removed because it reads as a perimeter
  line against the desktop wallpaper.
- Both runtime panels and the static Claude review surface use diffuse,
  negative-spread ambient shadows to retain depth without a halo or outline.

## Verification

- Add frontend CSS contract tests for the edge-aligned settings body and
  diffuse borderless shadows.
- Run the focused contract tests, the full frontend test/build checks, the
  Rust checks, the debug Tauri build, `git diff --check`, and the Impeccable
  detector on the changed UI files.
