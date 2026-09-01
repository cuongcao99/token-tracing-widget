# Token Tracing frontend visual baseline

Captured 2026-09-01 from the clean `refactor/ui-ux` checkout before the CSS
Modules refactor. This baseline uses the real React entry points and production
CSS through the ignored `harness.html` fixture page. It does not edit tracked
source, invoke native settings persistence, inspect provider records, or read
any source file contents. The only displayed roots are synthetic fixture paths.

## Reproduction

From `E:\AI thingy\Project\Personal\Token tracing widget`:

```powershell
npm run dev -- --host 127.0.0.1 --port 4177
```

The server is intentionally still running for the after-refactor comparison.
The browser session is the Codex in-app browser with browser ID
`-24f8-41b5-a8cb-6b4491a8615a` and tab `1`. The current page is:

```text
http://127.0.0.1:4177/.impeccable/frontend-modularity/visual/harness.html?surface=settings&mode=dark
```

Use the same page with these query parameters:

```text
surface=widget&mode=light&providers=0
surface=widget&mode=light&providers=1
surface=widget&mode=light&providers=2
surface=widget&mode=dark&providers=0
surface=widget&mode=dark&providers=1
surface=widget&mode=dark&providers=2
surface=settings&mode=light
surface=settings&mode=dark
```

Set the viewport to the dimensions encoded in each filename before taking a
viewport screenshot. The harness can be rerun after the CSS Modules refactor;
its source imports remain `/src/main.tsx` and `/src/settings-main.tsx`, so the
same browser page exercises the runtime surfaces rather than a static mock.

## Fixture and environment

- Fixed clock: `2026-09-01T12:00:00+07:00` (`Date.now()` is pinned in the
  harness).
- Usage state: `active`.
- Claude: `active`, session `12,480`, today `38,240`, updated
  `2026-09-01T11:57:00+07:00` (`3 min ago`).
- Codex: `idle`, session `6,320`, today `12,640`, updated
  `2026-09-01T11:48:00+07:00` (`12 min ago`).
- Total: `50,880`.
- Source health: Claude `detected` / Ready; Codex `limited` / Limited.
- Synthetic roots: `C:\Fixture\Claude\projects` and
  `C:\Fixture\Codex\sessions`.
- Persisted fixture theme: Claude; dark mode follows the `mode` query; both
  providers are initially visible unless `providers=0` or `providers=1` is
  supplied.
- Font resolution: the app's documented system stack from `tokens.css` (the
  browser has no bundled or downloaded product font).
- Browser: Codex in-app Browser, fixed viewport override, device scale 1,
  normal screenshot (not full-page).
- Viewport sizes: widget normal `440x192`, `440x244`, `440x316`; widget narrow
  `360x192`, `360x244`, `360x316`; settings normal `520x600`; settings narrow
  `380x600`.

The Tauri mock supports only the commands used by the entries: usage/settings
reads, harmless update responses, null source picking, event listen/unlisten/
emit, and no-op window sizing/drag/close calls. Unknown commands throw. The
mock never contacts the native runtime.

## Screenshot inventory

All files are ignored artifacts in this directory.

### Widget

- `baseline-widget-light-providers-0-normal-440x192.png`
- `baseline-widget-light-providers-0-narrow-360x192.png`
- `baseline-widget-light-providers-1-normal-440x244.png`
- `baseline-widget-light-providers-1-narrow-360x244.png`
- `baseline-widget-light-providers-2-normal-440x316.png`
- `baseline-widget-light-providers-2-narrow-360x316.png`
- `baseline-widget-dark-providers-0-normal-440x192.png`
- `baseline-widget-dark-providers-0-narrow-360x192.png`
- `baseline-widget-dark-providers-1-normal-440x244.png`
- `baseline-widget-dark-providers-1-narrow-360x244.png`
- `baseline-widget-dark-providers-2-normal-440x316.png`
- `baseline-widget-dark-providers-2-narrow-360x316.png`

### Settings

- `baseline-settings-light-normal-520x600.png`
- `baseline-settings-light-narrow-380x600.png`
- `baseline-settings-dark-normal-520x600.png`
- `baseline-settings-dark-narrow-380x600.png`

## Visual load check

All 16 screenshots loaded the expected real surface. The widget shows the
expected 0/1/2-provider height states without clipping at normal and narrow
widths. Settings renders both themes; narrow mode applies the existing
`max-width: 420px` behavior and keeps the synthetic roots visibly bounded.
