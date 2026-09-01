# Native Windows smoke report

Date: 2026-09-01 (Asia/Saigon)
Executable: `src-tauri/target/debug/token-tracing-widget.exe`
Checkout: `refactor/ui-ux` (clean before and after smoke)

## Observed

- The debug executable launched successfully from the repository working directory. The target became uniquely discoverable through Computer Use as `Token Tracing Widget`.
- The widget rendered at the configured 440 x 316 logical-pixel size. The screenshot showed the frameless dark overlay with transparent rounded corners, the `Token Tracing` heading, Claude and Codex provider rows, state labels, session/today metrics, and a total. The rendered values were normalized token metadata only; no prompts, responses, tool payloads, credentials, raw records, or arbitrary file contents were opened.
- The widget accessibility tree exposed `Move widget window` and all eight resize controls: top, top-right, right, bottom-right, bottom, bottom-left, left, and top-left. A grip click and a right-edge resize hit-zone click both returned a fresh screenshot and accessibility state without a crash. The right-edge hit-zone click left the window at 440 x 316; no persistent setting was changed.
- The widget surface had no visible native title bar or outer drop shadow. Taskbar enumeration did not expose a widget entry during the run.
- `Alt+F4` on the main widget completed without an error and hid the main window, matching the tray utility close behavior.
- The two executable processes created during this smoke run were both removed after the graceful close attempt failed to terminate them. A final process check found no `token-tracing-widget` process.

## Unobservable or not exercised

- The system tray icon/menu was not exposed as a target by the available Computer Use API (`list_apps`/`list_windows` returned windows only). Therefore Settings could not be opened through its supported tray path in this run.
- Settings rendering, scrolling, close behavior, settings elevation, the settings resize controls, source-folder picker cancel flow, and widget/settings cross-window preview were not exercised.
- Non-topmost behavior and DPI variants were not independently verified. The smoke run used the default exposed 440 x 316 widget state only.

## Cleanup and scope

- No source, dependency, persisted setting, theme, provider visibility, or source-root files were modified.
- No provider record or arbitrary file content was read.
- No screenshot files were persisted; the Computer Use observations were reviewed in place. This report is the only artifact written under `.impeccable/frontend-modularity/native-smoke/`.
