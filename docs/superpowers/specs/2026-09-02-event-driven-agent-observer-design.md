# Event-driven agent observer design

**Date:** 2026-09-02  
**Status:** Superseded by `docs/superpowers/specs/2026-09-02-file-observer-design-update.md`
**Supersedes:** The live-trigger and transient-trace portions of
`docs/superpowers/specs/2026-09-02-agent-hooks-design-update.md`.

## Decision

`Stop` is the terminal signal for one active trace run. `SessionEnd` remains
an idempotent cleanup signal. A run is keyed by `(provider, session_id)` and
uses `turn_id` as a generation fence when the provider supplies it.

The app keeps a permanently available, lightweight hook ingress while it is
running. Provider source observers and collection work are dynamic:

```text
activate hook -> register run -> publish Active -> start provider observer
               -> enqueue collection
observer event -> enqueue collection
stop hook     -> unregister run -> publish Idle when appropriate
               -> stop observer when its provider lease count reaches zero
               -> enqueue one final flush
```

The control path and collection path are separate. Hook handling never waits
for a blocking filesystem scan or SQLite transaction.

## Preserved boundaries

- Rust remains the owner of provider files, adapters, validation, deltas,
  deduplication, SQLite, diagnostics, and summary composition.
- Hooks remain lifecycle hints and never create usage events or token totals.
- The existing `CollectionCoordinator::collect` remains the only accounting
  path and its event/checkpoint transaction remains unchanged.
- The existing `sessions`, `usage_events`, `file_checkpoints`, `sources`,
  `settings`, and `diagnostics` schema remains unchanged.
- Hook stdin is bounded and projected to the allow-listed signal. Raw hook
  fields never enter IPC, SQLite, diagnostics, frontend payloads, or logs.
- React continues to receive only `UsageSummary` through
  `usage-summary-changed`; no frontend polling or new wire contract is added.

## Runtime modules

### Hook ingress

`HookListener` receives validated `TraceSignal` values from the local named
pipe. The hook executable remains fail-open: it exits quickly with code zero,
emits no output, and does not start the app. The sender performs a bounded
retry when the listener is temporarily busy. The listener uses the existing
bounded payload validation and forwards only validated signals.

### Trace control loop

The control loop owns the `TraceActivityRegistry`, the dynamic
`SourceObserver`, and a sender to the collection worker. It handles only
short-lived state transitions and never calls `collect_once`.

The registry is in memory only:

```text
TraceActivityRegistry
  active_runs: Map<(Provider, SessionId), TraceRun>
  hooked_providers: Set<Provider>
  next_generation: monotonic counter
```

On activation, an existing key is refreshed instead of spawning a duplicate
run. On stop, a supplied `turn_id` must match the current generation when
both are known; otherwise the stop is ignored as stale. A missing identifier
uses one provider-level anonymous slot.

The registry exposes a snapshot to summary composition. A provider is
hook-active when it has at least one active run. Once a provider has received
a hook, an empty active-run set makes that provider idle until its next
activation, even if a recent token event would otherwise satisfy the legacy
120-second event window. Providers that have not received a hook in the
current app process keep the existing event-time fallback.

### Source observer

`SourceObserver` starts one native directory observer per provider only when
that provider has at least one active run. Multiple runs for one provider
share the observer through a provider lease. The observer emits only generic
provider signals and never forwards paths or filenames.

There is no observer or provider collection schedule when no run is active.
While one or more runs are active, native notifications trigger coalesced
collection and a 30-second reconciliation remains as a repair mechanism for
missed notifications, overflow, rotation, and provider filesystem quirks.

### Collection worker

One collection worker remains the only SQLite writer. It receives activation,
observer-change, final-flush, configuration, and shutdown commands. It keeps
the existing bounded debounce and retry behavior, but reconciliation is
armed only while there is an active provider run.

The worker may finish a collection already in progress after a stop. It then
performs the one final flush requested by that stop. Final-flush output is
composed with the latest lifecycle snapshot so a just-committed token record
cannot resurrect `Active` after the run has ended.

## Lifecycle mapping

| Provider event | Lifecycle | Runtime effect |
|---|---|---|
| Claude `UserPromptSubmit` | activate | Start or refresh the run |
| Codex `SessionStart` | activate | Start the run before the first prompt |
| Codex `UserPromptSubmit` | activate | Start or refresh the run |
| Claude/Codex `Stop` | stop | End the current run |
| Claude `StopFailure` | stop | End the current run |
| Claude/Codex `SessionEnd` | stop | End all matching run state idempotently |

The stop path is immediate for presentation and observer ownership. A final
collection is a bounded data-integrity step, not a maintained observer.

If a provider omits a stop event, the registry expires the run after the
existing 120-second failsafe. The expiry is a deadline for active work, not a
busy polling loop.

## Time and data rules

- Hook receipt uses `Instant` for ordering, generation fencing, and failsafe
  deadlines. It is never persisted.
- Hook `observed_at` is only lifecycle metadata and never becomes
  `last_updated_at`.
- Token `observed_at` comes from the provider record and remains the only
  timestamp used for token totals, last token update, and event-time fallback.
- File modification time is an observer trigger/checkpoint input, not a token
  event timestamp.
- Activate publishes an immediate summary with existing totals and lifecycle
  `Active`; the post-commit collection summary may then update totals.
- Stop publishes lifecycle `Idle` immediately when no run remains for the
  selected provider; final-flush output cannot override that state.

## Restart behavior

When the app is closed, the hook exits successfully without persisting a
signal. On startup, SQLite checkpoints and usage events restore totals and
the existing initial collection repairs the index. No observer is inferred
from old lifecycle state. The next activate hook starts live observation.
This keeps lifecycle state ephemeral and avoids persisting sensitive hook
identities.

## Multi-session behavior

The registry supports multiple Claude or Codex runs at once. The observer is
shared per provider, while collection remains serialized through one
coordinator and one SQLite writer. Stopping one run releases only its lease;
the provider observer remains alive while another run is active.

Claude currently exposes a session identity in normalized observations. The
current Codex adapter emits no source session identity and uses file identity
as the effective session key. Therefore multi-session Codex collection is
safe and totals remain separated by file, but exact hook-session-to-token-file
association is not claimed by this slice. No hook identity is inserted into
the database as a fabricated Codex session key.

## Failure behavior

- Hook delivery failure remains fail-open for the provider and is repaired by
  the next activation or filesystem reconciliation while a run is active.
- Observer startup failure is provider-scoped and does not stop hook control,
  the other provider, or final collection.
- Collection/storage failure publishes no uncommitted summary and uses the
  existing bounded retry policy.
- Duplicate activate/stop/session-end signals are idempotent.
- A stale stop from an older turn cannot terminate a newer generation when
  both turn identities are available.

## Verification seams

- Registry: activate, refresh, duplicate signals, matching and stale stop,
  multiple sessions, provider lease transitions, expiry, and session-end
  cleanup.
- Control/collection isolation: a blocked collection does not delay hook
  state publication.
- Observer: dynamic start/stop, shared provider lease, provider isolation,
  path-free signals, and clean cancellation.
- Collection: activation trigger, coalescing, final flush, no post-stop
  resurrection, retry, reconciliation, restart, and no duplicate events.
- IPC: bounded retry, listener shutdown, malformed/oversized payloads, and
  no raw-field retention.
- Existing frontend, Rust, privacy, and integrated Windows build gates.
