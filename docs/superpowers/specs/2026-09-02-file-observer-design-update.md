# File-observer activity design update

**Date:** 2026-09-02
**Status:** Approved implementation direction
**Supersedes:** `2026-09-02-agent-hooks-design-update.md` and its hook-driven
lifecycle path.

## Decision

Token Tracing does not install, read, or depend on provider lifecycle hooks.
The native source observer is the only live trigger:

```text
enabled source root -> native file observer -> debounced collection
session file -> provider reader/delta/deduplication -> SQLite -> summary event
```

The observer starts for every enabled provider when the app starts, remains
alive while that source is enabled, and stops when the source is disabled or
the app shuts down. A stale `--hook` invocation exits successfully without
starting a second app instance so old user configuration is harmless.

## Activity contract

`UsageState::Active` is derived only from persisted, valid token events. A
provider/session is active when its newest event is strictly less than 10
seconds old relative to the collection clock. At the 10-second boundary the
session is idle. A file notification schedules a bounded 200ms collection;
the collector also schedules one activity-expiry refresh at the newest active
event's 10-second boundary. The expiry refresh publishes Idle even when no
new file notification arrives.

The frontend's Active phrase is presentation-only. It changes phrase every 15
seconds while Active and continues to respect reduced-motion preferences; it
does not affect collection, activity state, or token totals.

## Restart and closed-app behavior

On startup, the app first publishes the existing loading state, performs the
normal bounded historical collection, and then starts observers for enabled
roots. The SQLite index and file checkpoints remain the recovery authority, so
events written while the app was closed are collected on the next startup.
If a source is already current at startup, the initial collection derives
Active from its event timestamps; otherwise the result is Idle until the
observer reports a new file change or the bounded reconciliation catches it.

Closing the app stops observers and the live collection worker. No hook process,
pipe, lifecycle cache, or extra activity table is needed.

## Persistence and scalability

The database schema and accounting operations are unchanged. Existing event
deduplication, cumulative-to-delta conversion, checkpoints, source health, and
daily/session aggregates remain the source of truth. Activity expiry is an
in-memory scheduler deadline only.

The controller tracks observed providers as a set and routes each observer
notification by provider. This keeps the design compatible with multiple
simultaneous active sessions: collection reads all enabled roots, groups valid
events by provider/session, and derives the aggregate from each session's
latest event.

## Implementation map

- `src-tauri/src/sources/file_watcher.rs`: native per-provider observer.
- `src-tauri/src/app/live_collection.rs`: observer lifecycle, debounce,
  reconciliation, retry, and 10-second activity expiry.
- `src-tauri/src/usage/session_summary.rs`: strict 10-second activity rule.
- `src-tauri/src/app/runtime.rs`: historical collection and summary state;
  no transient hook overlay.
- `src/hooks/useActivityPhrase.ts`: 15-second Active phrase rotation.
- `src-tauri/src/database/`: unchanged; no lifecycle-hook data is persisted.
