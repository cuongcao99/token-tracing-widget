# Windows Agent Token Tracing Widget Design

**Date:** 2026-08-29
**Status:** Approved design

## Context

This is a new Windows 11 desktop project. Version 1 will provide a small desktop overlay that tracks token usage from Claude Code and Codex without modifying either agent's configuration.

The app is local-only. It reads supported local session files, normalizes token counters, stores a sanitized local index, and displays the latest agent, current-session total, today's total, and last-update time.

## Goals

- Run as a lightweight Windows 11 overlay with a system tray.
- Track Claude Code and Codex from local session data.
- Support Claude Code running natively on Windows or inside WSL.
- Show the latest active agent, current-session tokens, today's tokens, and last update.
- Preserve totals across restarts using a local SQLite index.
- Keep provider-specific parsing isolated behind stable internal interfaces.
- Store token metadata only and never retain conversational content.

## Non-goals

- macOS, Linux desktop, or Windows 10 support.
- Charts, history views, filters, model details, cost estimates, or token breakdowns in the overlay.
- Cloud sync, accounts, telemetry, remote APIs, or a local HTTP server.
- Installing hooks or changing Claude Code or Codex settings.
- Reading or storing prompts, responses, reasoning, tool payloads, credentials, or repository contents.
- Automatically discovering or launching WSL distributions.

## Technology decision

Use the following stack:

- **Desktop shell:** Tauri 2
- **Native core:** Rust
- **UI:** React, TypeScript, and Vite
- **Styling:** plain CSS with locally scoped component styles
- **Storage:** SQLite accessed only by the Rust core
- **Communication:** typed Tauri commands for requests and Tauri events for live summaries
- **Runtime shape:** one application executable with no app-managed Python, Node.js, or service sidecar; Windows WebView2 may use operating-system-managed helper processes

Tauri is preferred over Electron because the product prioritizes low idle resource use and a small distribution. It is preferred over WinUI 3 because React and web styling make the custom overlay faster to iterate while Rust still owns operating-system, filesystem, and storage work.

No frontend state library, CSS framework, ORM, or background service is needed for version 1.

## Architecture

The application has six bounded units.

### 1. App shell

The Tauri shell owns process lifecycle, the overlay window, system tray, single-instance enforcement, and Windows startup registration.

The overlay is transparent, frameless, always-on-top by default, and absent from the taskbar. Closing the overlay hides it. Only the tray's Quit command terminates the collector.

### 2. Source discovery

Source discovery resolves enabled provider roots without scanning unrelated user directories.

- Native Windows roots use known provider defaults under the current user's profile.
- Each provider has an optional explicit root override.
- WSL Claude Code support uses an explicit user-selected UNC folder, such as a path under `\\wsl.localhost\<distribution>\home\<user>`. The app does not invoke `wsl.exe` or enumerate distributions automatically.
- A configured root is validated before collection and can fail independently of the other provider.

Exact provider default paths and record shapes are compatibility details. Implementation begins with metadata-only probes against installed versions, then locks each observed format into sanitized test fixtures.

### 3. Provider adapters

Claude Code and Codex each implement the same adapter interface:

```text
discover_sessions(root, time_scope) -> sessions
read_observations(session, checkpoint) -> observations + next_checkpoint
```

Adapters may understand different file layouts and token semantics, but they may emit only normalized metadata:

```text
ProviderObservation
  provider
  source_session_key
  source_event_key
  observed_at
  counter_kind: incremental | cumulative
  input_tokens?
  cached_input_tokens?
  output_tokens?
  total_tokens
```

Adapters never return raw record bodies to the rest of the application. Unknown record kinds and unknown fields are ignored.

### 4. Collection core

The collection core owns validation, ordering, delta calculation, deduplication, checkpoints, and active-session selection.

Rules:

- Token counters must be non-negative integers.
- When all components are available, `total_tokens = input_tokens + output_tokens`.
- Cached input is metadata about input and is never added to total again.
- Incremental observations are stored once.
- Cumulative observations are ordered by timestamp and converted into deltas. Cumulative values are never summed directly.
- A cumulative counter decrease starts a new monotonic segment instead of producing a negative delta.
- Stable source event keys prevent duplicates after restart, rescan, truncation, or file rotation.
- The provider with the newest valid token event is active for two minutes. After two minutes without a new event, the overlay shows Idle and preserves the last-update time.
- Today's total combines all enabled providers and uses the current Windows local calendar day.

### 5. Storage

SQLite is a performance index and the source of restart-safe aggregates. Rust is its only caller; the React layer has no direct database access.

Logical tables:

- `sources`: provider, configured root, enabled state, and last health state.
- `sessions`: provider, opaque source session key, start time, and last activity time.
- `usage_events`: opaque source event key, session key, timestamp, and normalized token deltas.
- `file_checkpoints`: source file identity, byte offset, size, modification time, and monotonic segment state.
- `settings`: overlay and source preferences.
- `diagnostics`: bounded sanitized error category, provider, count, and last occurrence.

The database does not store session file contents, raw JSON, prompts, responses, reasoning, tool payloads, credentials, repository paths, or working directories. Explicit source-root overrides are stored locally because collection requires them.

Clearing the local index requires confirmation. It removes normalized events, checkpoints, and diagnostics but keeps user settings. Collection then rebuilds only the current-session and current-day scope.

### 6. Presentation

Rust exposes a small UI contract:

```text
UsageSummary
  state: loading | active | idle | unavailable | stale
  provider?
  current_session_tokens?
  today_tokens
  last_updated_at?
  source_health[]
```

The React UI can request the current summary and subscribe to summary-changed events. It cannot request raw observations or arbitrary files. The settings window can read and replace only the explicitly configured provider roots.

## Data flow

1. App starts and resolves enabled source roots.
2. SQLite loads checkpoints and known event identities.
3. On first run, each adapter scans only files needed to identify the latest session, events from the current local day, and the nearest preceding cumulative observation needed to establish the day's baseline.
4. A filesystem watcher tails changed files. A 30-second reconciliation pass covers missed events and unreliable notifications, including WSL UNC paths.
5. Adapter emits normalized provider observations.
6. Collection core validates, orders, deduplicates, and converts observations into deltas.
7. Usage events and file checkpoints commit in one SQLite transaction.
8. Aggregator computes current-session and current-day totals.
9. Tauri emits a `UsageSummary` only after commit succeeds.
10. React updates the overlay without polling the database.

The collector uses incremental byte offsets. An incomplete final JSONL line remains pending until a later append completes it.

## Interface

The overlay is approximately 320 by 120 logical pixels and can be dragged. Its last position is remembered per monitor.

Content:

- Header: provider name and Active, Idle, or error state.
- Primary value: current-session total tokens.
- Secondary value: today's total tokens.
- Footer: relative last-update time.

Version 1 has no chart, session list, model name, cost, or expanded token breakdown.

The tray menu contains Show/Hide, Settings, and Quit. The settings window is created only while open and contains:

- Launch on Windows login.
- Always on top.
- Opacity.
- Reset overlay position.
- Automatic native paths and optional provider root overrides.
- Clear local index with confirmation.

## Error handling

- A missing installation reports Not detected for that provider.
- A permission failure reports the affected configured root and a recovery action; the other provider continues.
- A malformed, partial, or unknown record cannot crash collection. Partial final lines wait; malformed complete records increment a sanitized diagnostic.
- File truncation or rotation resets the byte offset and relies on event deduplication during rescan.
- Reconciliation repairs missed watcher events every 30 seconds.
- An unsupported provider schema marks only that adapter Unsupported format and retains last valid totals.
- A SQLite write failure keeps the overlay alive with a stale state and retries with bounded backoff.
- Suspected database corruption is never silently deleted. The UI offers an explicit backup-and-rebuild action.
- Diagnostic logs contain categories and counts, not raw session data.

## Security and privacy

- Rust owns all filesystem and SQLite access.
- Tauri capabilities grant only commands required by the overlay and settings UI.
- The overlay webview receives typed summaries, never file paths or raw provider records. The settings window can receive configured provider roots but cannot read arbitrary files.
- No network permission or network client is included in version 1.
- No telemetry is collected.
- Source parsing uses allow-listed numeric fields and bounded strings.
- Database queries use parameters.
- Synthetic fixtures replace identifiers and timestamps and contain no conversational content.

## Testing strategy

### Rust unit tests

- Claude Code and Codex parsing.
- Normalization and validation.
- Incremental and cumulative counter handling.
- Counter resets and monotonic segments.
- Deduplication and active/idle selection.

### Integration tests

- First scan and incremental append.
- Partial final line followed by completion.
- File rotation, truncation, and restart.
- Transactional event/checkpoint persistence.
- Concurrent updates from both providers.
- Native and WSL-style source roots using temporary test directories.
- Local midnight rollover and clock changes.
- Database failure and recovery states.

### UI tests

- Loading, active, idle, unavailable, and stale states.
- Large token values and missing current-session values.
- Tray commands and settings validation at the command boundary.
- Rejection of payloads containing forbidden raw fields.

### Windows smoke tests

- Transparent frameless rendering.
- Dragging and remembered multi-monitor position.
- Always-on-top and taskbar behavior.
- Tray Show/Hide, Settings, and Quit behavior.
- Launch-on-login setting.
- Installed build startup and clean uninstall.

### Performance acceptance

- No continuous busy loop while sources are idle.
- Existing large session files are processed incrementally after their first checkpoint.
- Overlay updates do not require rescanning source directories or querying raw session files.

## Acceptance criteria

Version 1 is complete when:

1. One Windows 11 installer starts one Tauri application executable with no app-managed sidecar or background service.
2. Native Codex and native or explicitly configured WSL Claude Code sources can be enabled independently.
3. New valid token events update the overlay without manual refresh.
4. Overlay shows provider state, current-session total, today's total, and last-update time.
5. Totals remain correct after restart, duplicate scans, partial writes, and file rotation.
6. Cumulative snapshots are converted to deltas and never summed directly.
7. Missing, blocked, malformed, or changed sources degrade independently without crashing the app.
8. SQLite and logs contain no prompts, responses, reasoning, tool payloads, credentials, or repository contents.
9. Tray, settings, positioning, startup, and uninstall pass Windows 11 smoke testing.
10. Unit, integration, UI, and privacy-boundary tests pass.

## References

- [Tauri configuration and window options](https://v2.tauri.app/reference/config/)
- [Tauri system tray](https://v2.tauri.app/learn/system-tray/)
- [Tauri security permissions](https://v2.tauri.app/learn/security/using-plugin-permissions/)
- [React with TypeScript](https://react.dev/learn/typescript)
- [Claude Code Windows setup](https://docs.anthropic.com/en/docs/claude-code/getting-started)
- [Microsoft Windows app framework guidance](https://learn.microsoft.com/en-us/windows/apps/)
- [Electron process model](https://www.electronjs.org/docs/latest/tutorial/process-model)
