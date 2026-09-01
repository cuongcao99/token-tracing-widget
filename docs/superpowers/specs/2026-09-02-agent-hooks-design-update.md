# Agent lifecycle hooks design update

**Date:** 2026-09-02  
**Status:** Approved implementation direction  
**Supersedes:** The hook-installation non-goal in the 2026-08-29 design for
the optional tracing path only.

## Decision

Token Tracing may install an explicitly consented, user-scope command hook for
Claude Code and Codex. The hook is a lifecycle trigger, not a token-usage
source:

```text
provider hook -> bounded metadata projection -> local pipe -> live collector
provider session file -> existing reader/delta/deduplication -> SQLite totals
```

The existing filesystem reader remains mandatory for token accounting and
restart recovery. Hooks never create `UsageEvent` rows and never bypass the
collection transaction.

## Lifecycle contract

The app accepts only a versioned `TraceSignal` containing the provider,
allow-listed provider event, normalized lifecycle (`start_or_continue`,
`pause`, or `stop`), a locally generated timestamp, bounded opaque session and
turn identities, and an optional sequence. Unknown fields, invalid values, and
oversized input are rejected. Prompt text, assistant text, transcript paths,
working directories, model data, tool data, credentials, and arbitrary hook
fields are discarded before IPC.

`UserPromptSubmit` and `SessionStart` start or continue activity. `Stop` and
`StopFailure` pause a turn. `SessionEnd` stops a session. The lifecycle hint is
ephemeral and may make the overlay show `Active` before the next token record
arrives; it never changes `lastUpdatedAt` or token totals. A missing or late
hook is repaired by filesystem notifications and 30-second reconciliation.

## Process and privacy boundary

The installed executable has two modes:

- normal mode starts the Tauri app and its in-process collector;
- `--hook <provider>` reads at most a bounded stdin payload, projects the
  allow-listed fields, attempts a local authenticated/access-controlled named
  pipe send, emits no stdout/stderr, and exits successfully even when the app
  is not running.

The main process owns the named-pipe listener and forwards only the validated
signal into the existing live-collection channel. The listener is stopped and
joined during app shutdown. A pipe signal can trigger collection but cannot
write SQLite directly.

The pipe relies on the Windows default security descriptor for a pipe created
by the current user; the signal is still treated as an untrusted hint. A fake
signal can at most affect transient activity presentation or cause a bounded
reconciliation attempt, never token totals.

## Installation and restart behavior

Installation edits only the user-scope provider configuration and merges the
app-owned command into existing hook groups idempotently. Unrelated hooks and
unknown configuration keys are preserved. Uninstall removes only the exact
app-owned command and refuses to overwrite malformed configuration. Claude
uses `~/.claude/settings.json`; Codex uses `~/.codex/hooks.json` with
`commandWindows`. Codex trust remains provider-owned: installed does not mean
trusted or active, and the app never uses a trust bypass.

When the app is closed, hook delivery fails open and no signal is persisted.
On restart, the existing SQLite index, checkpoints, deduplication, and initial
collection recover totals from session files. The next hook or file change
then resumes live updates.

## Scope limits

- Windows 11 native executable hooks are the supported path.
- WSL Claude hook invocation remains an explicit fallback/validation item.
- No network client, service, sidecar, raw hook log, raw session payload, or
  token accounting through hooks is introduced.
- Codex `/hooks` review is a required user action when Codex marks the command
  untrusted.

## Implementation map

- `src-tauri/src/types/trace_signal.rs`: versioned signal and payload
  projection/validation.
- `src-tauri/src/app/trace_signal.rs`: hook-mode dispatch, named-pipe ingress,
  and shutdown seam.
- `src-tauri/src/app/live_collection.rs`: forward lifecycle signals to the
  ephemeral runtime hint and retain existing scheduling/reconciliation.
- `src-tauri/src/app/runtime.rs`: apply and expire transient lifecycle hints
  without changing persisted aggregates.
- `src-tauri/src/hooks_config.rs`: pure provider configuration merge/remove
  helpers.
- `src-tauri/src/commands/trace_hooks.rs`: consented install/remove/status
  command boundary.
- `src-tauri/src/types/trace_hooks.rs`: frontend-safe hook status contract.
- `src-tauri/src/database/`: unchanged for lifecycle signals; no raw hook
  payload or hook identity is persisted.

