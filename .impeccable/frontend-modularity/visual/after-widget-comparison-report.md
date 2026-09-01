# D1 widget visual comparison

Captured 2026-09-01 from the running Vite harness at
http://127.0.0.1:4177/.impeccable/frontend-modularity/visual/harness.html.
This is a browser-only comparison of the staged D1 frontend against the
preserved baseline. The harness used the real /src/main.tsx entry, the fixed
clock and synthetic metadata-only Tauri responses described in
baseline-report.md, and no network or native settings persistence.

## Result

No visual regression was found in the 12 gated widget cases. Every AFTER file
is byte-for-byte identical to its BASELINE pair. Therefore each pair has
differing bytes = 0, differing pixels = 0, mean absolute RGBA difference = 0,
and max absolute channel difference = 0.

| Case | Viewport | Differing bytes | Differing pixels | Mean abs RGBA | Max channel |
| --- | ---: | ---: | ---: | ---: | ---: |
| light, providers=0, normal | 440x192 | 0 | 0 | 0 | 0 |
| light, providers=0, narrow | 360x192 | 0 | 0 | 0 | 0 |
| light, providers=1, normal | 440x244 | 0 | 0 | 0 | 0 |
| light, providers=1, narrow | 360x244 | 0 | 0 | 0 | 0 |
| light, providers=2, normal | 440x316 | 0 | 0 | 0 | 0 |
| light, providers=2, narrow | 360x316 | 0 | 0 | 0 | 0 |
| dark, providers=0, normal | 440x192 | 0 | 0 | 0 | 0 |
| dark, providers=0, narrow | 360x192 | 0 | 0 | 0 | 0 |
| dark, providers=1, normal | 440x244 | 0 | 0 | 0 | 0 |
| dark, providers=1, narrow | 360x244 | 0 | 0 | 0 | 0 |
| dark, providers=2, normal | 440x316 | 0 | 0 | 0 | 0 |
| dark, providers=2, narrow | 360x316 | 0 | 0 | 0 | 0 |

The 12 AFTER artifacts are:

- after-widget-light-providers-0-normal-440x192.png
- after-widget-light-providers-0-narrow-360x192.png
- after-widget-light-providers-1-normal-440x244.png
- after-widget-light-providers-1-narrow-360x244.png
- after-widget-light-providers-2-normal-440x316.png
- after-widget-light-providers-2-narrow-360x316.png
- after-widget-dark-providers-0-normal-440x192.png
- after-widget-dark-providers-0-narrow-360x192.png
- after-widget-dark-providers-1-normal-440x244.png
- after-widget-dark-providers-1-narrow-360x244.png
- after-widget-dark-providers-2-normal-440x316.png
- after-widget-dark-providers-2-narrow-360x316.png

## Browser inspection

- All requested 0/1/2-provider states rendered at the expected 192/244/316px
  heights at both 440px and 360px widths.
- document and widget-root overflow were false in all 12 captures.
- At 440x316, the widget root measured 440x316 with 16px radius,
  padding: 21px 22px 19px, box-shadow: none, and overflow: hidden.
  Light and dark root backgrounds were rgb(250, 249, 245) and
  rgb(24, 23, 21).
- The title remained 32px, weight 400, 38.4px line height, coral
  rgb(204, 120, 92), with the documented Copernicus/Tiempos/Garamond
  display stack. Provider and metric UI retained the documented StyreneB/Inter
  system stack.
- The six-dot move control remained present as Move widget window, measured
  28x18px with cursor: grab. The resize affordance remained a full-window
  container with all eight labeled native direction buttons; the first edge
  handle reported cursor: n-resize.
- A light 520x600 and dark 380x600 settings route smoke check still rendered
  the legacy settings surface with its compatibility globals. Both roots were
  visible and had no document overflow. No browser console warnings or errors
  were observed. No settings AFTER screenshots were created because the final
  D2 settings CSS slice is not ready.

## Evidence limits

These results establish browser fixture rendering at a device scale of
approximately 1 (window.devicePixelRatio was 1.0000000447034836). They do not
establish native Windows frameless transparency, taskbar visibility,
non-topmost behavior, actual native drag or resize, or other OS/DPI behavior.
The fixture only exposes active and idle providers, so loading/unavailable/
stale provider states were not part of this visual run.
