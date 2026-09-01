# Frontend modularity visual comparison plan

Status: prepared; after capture is held for the root agent's gate.

## Current verification surface

- Vite is serving the ignored harness at `http://127.0.0.1:4177/`.
- The harness imports the production React entries `/src/main.tsx` and
  `/src/settings-main.tsx`; it is not the static design preview.
- The IPC boundary is synthetic and metadata-only. The fixture uses a fixed
  clock (`2026-09-01T12:00:00+07:00`), fixed usage summaries, fixed source
  roots, and harmless mocked Tauri commands.
- The in-app Browser can connect and the harness loaded the settings surface
  at the URL recorded in `baseline-report.md`. The original user tab was not
  present in the current Browser session, so a fresh local tab was opened for
  read-only inspection.

## Required after-capture matrix

Keep the fixed clock, device scale 1, normal (not full-page) screenshots, and
the exact viewport dimensions from the baseline. Capture one after image for
each entry below only after the root agent explicitly gates the comparison.

| Surface | Theme | Fixture state | Normal | Narrow |
| --- | --- | --- | --- | --- |
| Widget | light | providers=0 | 440x192 | 360x192 |
| Widget | light | providers=1 | 440x244 | 360x244 |
| Widget | light | providers=2 | 440x316 | 360x316 |
| Widget | dark | providers=0 | 440x192 | 360x192 |
| Widget | dark | providers=1 | 440x244 | 360x244 |
| Widget | dark | providers=2 | 440x316 | 360x316 |
| Settings | light | default fixture | 520x600 | 380x600 |
| Settings | dark | default fixture | 520x600 | 380x600 |

The after filename must preserve the baseline stem and replace the
`baseline-` prefix with `after-`, leaving all 16 baseline images unchanged.

## Comparison checks

For each pair, record pixel difference statistics and inspect the rendered
surface for clipping, spacing, wrapping, border/background changes, provider
state colors, icon alignment, and font fallback. Also collect read-only DOM
geometry and computed styles for the key root/header/section/row/resize-handle
elements so a visible difference can be tied to a layout cause.

Exercise both widget provider counts in both themes, both settings themes,
normal and narrow widths, the settings scroll/sticky-header behavior, and
focus-visible states where the fixture exposes controls. Use only the existing
fixture data; do not invoke native source picking or persistence.

## Expanded browser-only cases

After the gated 16-image matrix, optionally probe additional CSS viewport
sizes: widget `320x192` and `520x316`; settings `320x600` and `600x600`.
These are responsive browser checks and must be labeled separately from the
baseline comparison. The Browser viewport capability accepts CSS width and
height only; no native Windows DPI or OS window scale evidence can be claimed.
Record the observed `window.devicePixelRatio` and keep the native frameless
drag, resize, transparency, taskbar, and topmost behavior explicitly outside
the browser conclusion.

## Safe fixture limits

The current harness exposes active/idle provider states only. It does not
provide a query-controlled unloaded/initial-loading fixture. If those states
cannot be reached through existing runtime controls without changing the
harness, mark them as untested instead of modifying tracked source or the
fixture. Browser inspection is read-only; baseline images and tracked source
remain untouched.
