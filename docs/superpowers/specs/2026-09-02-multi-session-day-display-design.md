# Multi-session day display

Date: 2026-09-02
Status: Approved in chat; written-spec review pending

## Summary

Expose the current local day's provider sessions in the existing usage
summary. Each provider keeps its existing aggregate metrics and additionally
shows the number of sessions, active sessions directly, and idle sessions
behind a native disclosure. A session displays its mutable name when known and
otherwise its stable opaque ID, followed by that session's current-day token
total.

This is a deliberate update to the earlier UI specs that kept session identity
internal. The existing Rust-owned collection, SQLite accounting, file observer,
privacy boundary, provider order, and summary event remain in place.

## Goals

- Show every provider's current-day session count.
- Show each current-day session's token total.
- Show active sessions without an extra interaction.
- Allow idle sessions to be expanded and collapsed.
- Prefer a session name, falling back to its ID.
- Keep session identity stable across token updates and name changes.
- Let the existing resizable widget use the width the user gives it.
- Preserve the current aggregate `Session` and `Today` metrics.

## Non-goals

- No history-day picker or historical session browser.
- No prompt, response, reasoning, tool payload, repository path, credential,
  raw provider record, or file-content exposure.
- No session editing, renaming, opening, or navigation action in the widget.
- No new provider, network client, telemetry, hook, polling loop, state
  library, CSS framework, ORM, or font package.
- No model, cost, or per-request breakdown.

## Domain semantics

### Day membership

The session list contains sessions with at least one accepted `UsageEvent`
whose timestamp belongs to the injected Windows local day being summarized.
The existing event validation, deduplication, delta conversion, and checkpoint
rules remain authoritative.

Each session's `todayTokens` is the sum of its validated token deltas for that
day. The provider `todayTokens` remains the aggregate of those same events, so
the session totals must add up to the provider total.

### Active and idle

`active` means the session's newest valid event is strictly less than
`ACTIVE_SESSION_WINDOW_SECONDS` old. The current constant remains 10 seconds;
an event exactly 10 seconds old is idle.

`idle` means the session belongs to the current day but does not meet the
active rule. A session may become idle without receiving a new event; the next
summary calculation derives that state from the clock and the latest event.

The existing provider `Session` aggregate is unchanged: sum current-day
tokens for active sessions; when none are active, retain the newest session's
current-day total. Provider state and last-update semantics also remain
unchanged.

### Ordering

Within each provider, active sessions appear first. Each group is ordered by
newest valid event descending, with the stable session ID as the deterministic
tie-breaker. Idle sessions use the same order when disclosed.

The provider session count is derived from the serialized session array; no
duplicate `sessionCount` field is added to the contract.

## Identity and names

`UsageEvent.session_key` remains the internal grouping identity. The frontend
receives that identity as an opaque `id`; it never receives the source file
path used to derive a Codex identity. The ID is required, non-empty, and
bounded at the Rust-to-frontend boundary.

The wire `name` is optional mutable metadata. The display rule is:

```text
display label = name when it is a valid non-empty name, otherwise id
```

Name changes do not create sessions, alter token totals, or reset active/idle
state. The same `id` remains the React key, so an arriving name or a rename
updates the label in place and preserves disclosure state. A missing name in a
later record does not erase a previously known name; only a newer valid name
replaces it. Name input is trimmed, control characters are rejected, and its
stored/wire length is bounded.

Current adapters do not expose a supported session name, so the first version
will normally render IDs. The optional field and persistence path make the
name-first rule effective when an adapter later supplies an allow-listed name
without changing the frontend contract.

Names are persisted in the existing `sessions` table as nullable metadata.
The schema change must be backward-compatible and idempotent for existing
SQLite databases. Updating a session's name must preserve its
`started_at`, `last_activity_at`, and all usage events.

## Backend contract

Add a focused `SessionUsageSummary` type in the existing `types` boundary:

```text
SessionUsageSummary {
  id: string
  name?: string
  state: active | idle
  todayTokens: u64
}
```

Extend `ProviderUsageSummary` with:

```text
sessions: SessionUsageSummary[]
```

The existing `get_usage_summary` command and `usage-summary-changed` event
continue to carry the complete typed `UsageSummary`; no second command or
frontend-side filesystem read is introduced. Provider summaries remain in the
canonical provider order. A provider with no current-day sessions exposes an
empty array and a zero session count.

The collection path must build session summaries from the same validated
events used by the aggregate provider summary. SQLite supplies the persisted
session name when present; it must not make raw source records available to
React. Any adapter-provided name is optional metadata only and must pass the
same bounded validation before persistence or serialization.

The frontend parser remains strict: it accepts only the documented session
fields, rejects missing/empty IDs, rejects unsafe token counts or malformed
states, and rejects unknown keys. Existing summary privacy tests remain
authoritative.

## Frontend behavior

The existing provider row keeps its aggregate `Session` and `Today` metrics and
adds a compact current-day session count in the provider section.

- Active session rows render immediately below the aggregate metrics.
- Idle rows are inside a native `<details>` disclosure, collapsed initially.
- The disclosure summary includes the idle count; zero idle sessions omit the
  disclosure.
- Every row contains the resolved label and its `todayTokens` value.
- IDs are stable keys. The label is recomputed from the latest `name` and
  `id`, so a name arrival or rename needs no special migration in React.
- The full resolved label remains available through the native `title` and an
  accessible label when CSS has to ellipsize it.

Width is layout-driven, not character-driven. Session rows use a flexible
label column and a max-content token column; the label may ellipsize only when
the actual widget width leaves insufficient room. No fixed character limit is
used for rendering. The existing 360-720 logical-pixel width range remains.

The widget grows with disclosed content up to the existing 520 logical-pixel
maximum. When more content exists than that bound, the session/provider list
scrolls inside the widget so disclosure never renders outside the native
window. Height updates use the existing native window-sizing bridge and UI
events/layout measurement only; no JavaScript polling loop is added.

## Ownership and files

Keep changes at the existing responsibility boundaries:

- `src-tauri/src/types/`: session and provider wire types.
- `src-tauri/src/usage/`: day filtering, ordering, active/idle state, and
  aggregate/session projection.
- `src-tauri/src/database/`: nullable session-name migration, upsert, and
  summary read.
- `src-tauri/src/providers/`: only allow-listed optional name extraction if a
  current provider record supplies one; never forward raw records.
- `src-tauri/src/collection/` and `src-tauri/src/commands/`: thread the typed
  projection through the existing summary path.
- `src/lib/contracts/`: strict session contract parsing.
- `src/lib/widget-view-model.ts`: map the typed session projection and derive
  count/active/idle view data.
- `src/components/widget/` and `src/styles/widget/`: focused session list,
  disclosure, responsive row layout, and accessibility.
- `src/lib/window-sizing.ts` plus its existing bridge only if the content-size
  calculation requires the current height API to accept the new derived
  layout state.

Do not move collection ownership into React and do not broaden the existing
provider/session domain types beyond this display slice.

## Acceptance checks

Rust tests cover:

- two active sessions for one provider, each with its own current-day total;
- active-first ordering and deterministic newest-event ordering;
- idle disclosure data and the exact-10-second boundary;
- previous-day events excluded from the session list;
- provider aggregate equals the sum of current-day session totals;
- a persisted name appearing and being replaced while ID, tokens, and state
  remain stable;
- existing databases gaining the nullable name column without data loss;
- serialized payloads containing only the approved session metadata.

Frontend tests cover:

- strict parsing of valid sessions and rejection of unsafe/unknown fields;
- name-first/ID-fallback view-model mapping;
- active rows visible, idle rows collapsed/expandable, and session counts;
- stable ID keys and updated labels after a rename;
- responsive flexible label/token columns without a hard-coded character cut.

Run the repository gates from `AGENTS.md`: frontend tests/build, Rust format/
check/tests, and the debug Tauri build for this cross-boundary change.

## Decision record

This spec supersedes the session-list and session-identity exclusions in:

- `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`;
- `docs/superpowers/specs/2026-08-30-claude-editorial-multi-provider-design-update.md`;
- `docs/superpowers/specs/2026-08-31-session-provider-theme-refactor-design-update.md`;
- `docs/superpowers/specs/2026-09-01-frontend-modularity-design-update.md`.

The 2026-09-02 file-observer design remains authoritative for lifecycle and
activity detection. The earlier hook/event-driven design remains superseded and
is not reopened by this feature.
