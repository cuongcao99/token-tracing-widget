# Multi-session day display Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add current-day per-session token rows to every provider while preserving the existing aggregate summary and local-only privacy boundary.

**Architecture:** Keep collection, session identity, name metadata, day filtering, and token aggregation in Rust/SQLite. Extend the existing typed `UsageSummary` contract with an optional session name and a per-provider session array, then map it into a small React view model. Render active rows directly and idle rows with native `<details>`, using the existing window-sizing bridge plus measured content height for expansion.

**Tech Stack:** Rust, Tauri 2, SQLite via `rusqlite`, React, TypeScript, CSS Modules, Vitest, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-09-02-multi-session-day-display-design.md`

## Global Constraints

- Keep version one local-only and Windows 11-only.
- Preserve metadata-only collection: prompts, responses, reasoning, tool payloads, credentials, repository contents, working directories, raw provider records, and arbitrary file contents never enter normalized events, SQLite, diagnostics, or frontend payloads.
- Keep filesystem, collection, source discovery, and SQLite access in Rust; React receives typed summaries only.
- The existing event validation, deduplication, delta conversion, checkpoint, provider order, and 10-second activity rules remain authoritative.
- Width is layout-driven, not character-driven; keep the existing 360-720 logical-pixel range and 520 logical-pixel maximum height.
- Add no network client, telemetry, sidecar, background service, frontend state library, CSS framework, ORM, font package, or new Tauri command.
- Work test-first and run frontend tests/build, Rust format/check/tests, and the debug Tauri build before completion.

---

### Task 1: Add the typed session metadata path and SQLite compatibility

**Files:**
- Create: `src-tauri/src/types/session_usage_summary.rs`
- Modify: `src-tauri/src/types/mod.rs`
- Modify: `src-tauri/src/types/token_observation.rs`
- Modify: `src-tauri/src/types/usage_event.rs`
- Modify: `src-tauri/src/types/provider_usage_summary.rs`
- Modify: `src-tauri/src/types/usage_summary.rs`
- Modify: `src-tauri/src/database/schema.rs`
- Modify: `src-tauri/src/database/sessions.rs`
- Modify: `src-tauri/src/database/usage_events.rs`
- Modify: `src-tauri/src/database/connection.rs`
- Modify: `src-tauri/src/usage/cumulative_delta.rs`
- Modify: all Rust test literals found by `rg -n "TokenObservation \{|UsageEvent \{" src-tauri/src src-tauri/tests`
- Test: `src-tauri/tests/database.rs`
- Test: `src-tauri/src/commands/usage_summary.rs`
- Test: `src-tauri/tests/widget_settings_contract.rs`

**Interfaces:**
- `SessionUsageState` is a serialized Rust enum with `active` and `idle` values.
- `SessionUsageSummary` serializes as `{ id: String, name?: String, state: SessionUsageState, todayTokens: u64 }` using camelCase field names and omitting only `name` when absent.
- `TokenObservation` and `UsageEvent` each gain `session_name: Option<String>`; `UsageEvent::from_delta` copies the normalized optional name from the observation.
- `ProviderUsageSummary::new` becomes `new(provider, state, current_session_tokens, today_tokens, last_updated_at, sessions)` and always serializes `sessions`, including an empty array.
- `IndexStore::query_events_for_summary(&self, day_start: &str, now: &str) -> Result<SummaryRows, StorageError>` returns events with the current persisted session display name joined onto each event.

- [ ] **Step 1: Write the failing persistence and contract tests.**

Add a database test that attaches a name to a normalized event, applies it, and verifies the summary query returns the name without exposing source-only columns:

```rust
#[test]
fn summary_query_round_trips_only_the_session_display_name() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = IndexStore::open(&directory.path().join("index.sqlite")).unwrap();
    let mut event = test_usage_event("event-named", "file-a");
    event.session_name = Some("Run alpha".to_owned());

    database
        .apply_batch(&CollectionBatch::new(
            vec![event],
            vec![FileCheckpoint::with_position("file-a", Provider::Claude, 42, 42)],
        ))
        .unwrap();

    let rows = database
        .query_events_for_summary("2026-01-01T00:00:00Z", "2026-01-01T00:00:01Z")
        .unwrap();
    assert_eq!(rows.events[0].session_name.as_deref(), Some("Run alpha"));
}
```

Add a migration test that creates the old `sessions` shape, reopens it through `IndexStore::open`, and verifies `PRAGMA table_info(sessions)` contains `display_name` and `display_name_updated_at` while the existing row remains intact. Update exact serialized-key assertions so a provider contains `sessions: []`.

- [ ] **Step 2: Run the focused tests to verify the contract fails before implementation.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test database summary_query_round_trips_only_the_session_display_name`

Expected: compile failure because `UsageEvent` has no `session_name` field yet.

- [ ] **Step 3: Define the session wire type and thread optional names through normalized events.**

Create the focused type and add the fields without changing provider record payloads:

```rust
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionUsageState {
    Active,
    Idle,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionUsageSummary {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub state: SessionUsageState,
    pub today_tokens: u64,
}
```

In the same module define `MAX_SESSION_LABEL_LENGTH: usize = 256`, `normalize_session_name(Option<&str>) -> Option<String>`, and `safe_session_id(&str) -> String`. The normalizer trims, rejects control characters, and drops names longer than the bound; `safe_session_id` returns a non-empty safe ID unchanged and otherwise returns a stable `session-<sha256-hex>` alias using the existing `sha2` dependency. Add `pub mod session_usage_summary;`, add `session_name: Option<String>` to `TokenObservation` and `UsageEvent`, copy the normalized optional name in `UsageEvent::from_delta`, initialize it to `None` in Claude/Codex parser records and every existing fixture/test literal, and add `sessions: Vec<SessionUsageSummary>` to `ProviderUsageSummary`. Update `UsageSummary::loading`, `UsageSummary::unavailable`, and every `ProviderUsageSummary::new` call to pass `Vec::new()` until the aggregation task supplies real rows. Current parsers must not inspect arbitrary record `name` fields; they continue emitting `None` unless a provider-specific allow-listed field is added with a parser test.

- [ ] **Step 4: Add an idempotent nullable metadata migration and name-preserving upsert.**

Extend newly created `sessions` tables with:

```sql
display_name TEXT,
display_name_updated_at TEXT
```

After the existing `CREATE TABLE IF NOT EXISTS` batch, inspect `PRAGMA table_info(sessions)` and issue `ALTER TABLE sessions ADD COLUMN ...` only for missing columns. This preserves old databases and is safe to run every startup.

Change `sessions::upsert` so it keeps the existing start/activity min/max rules and updates the name only when `event.session_name` is a valid non-empty trimmed string and `event.observed_at` is newer than or equal to `display_name_updated_at`. Invalid names are discarded as metadata while the token event remains accepted. Store the normalized name and its timestamp; never store a source path or raw record.

- [ ] **Step 5: Join persisted names into the existing normalized summary query.**

Keep `usage_events` unchanged as the token ledger. In `usage_events::query_between`, add a `LEFT JOIN sessions` on `(provider, session_key)`, select `sessions.display_name` after the twelve event columns, and populate `UsageEvent.session_name` from that nullable column. Do not add arbitrary provider fields to the query or `SummaryRows`.

- [ ] **Step 6: Run the focused Rust tests and commit the boundary change.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test database`

Expected: PASS, including the old-schema migration, named event query, atomic batch behavior, and privacy assertions.

Commit:

```text
feat: persist session display names
```

### Task 2: Project current-day active and idle session summaries

**Files:**
- Modify: `src-tauri/src/usage/session_summary.rs`
- Modify: `src-tauri/src/usage/provider_summary.rs`
- Modify: `src-tauri/src/collection/mod.rs` only where constructor/type threading requires it
- Test: `src-tauri/tests/session_summary.rs`
- Test: `src-tauri/tests/provider_summary.rs`
- Test: `src-tauri/tests/collection_core.rs`

**Interfaces:**
- `SessionAggregate` gains `session_key: String` and `name: Option<String>` while retaining `active`, `total_tokens`, `current_day_tokens`, and the private ordering fields.
- `compute_session_aggregation(events, now, local_day)` returns only sessions with at least one event on `local_day` when `local_day` is `Some`; its `sessions` are active-first and newest-first.
- `compute_provider_summary(provider, events, health, now, local_day)` maps those aggregates into `ProviderUsageSummary.sessions` while retaining the existing provider state, current-session fallback, today total, and last-update behavior.

- [ ] **Step 1: Write failing aggregation tests for day membership, names, and ordering.**

Add a test with one active and two idle same-provider sessions, including a previous-day event, and assert the output shape:

```rust
#[test]
fn projects_current_day_sessions_active_first_with_stable_order() {
    let mut renamed = UsageEvent::for_test(
        Provider::Claude,
        "session-b",
        "2026-01-01T00:00:05Z",
        22,
    );
    renamed.session_name = Some("Renamed run".to_owned());
    let events = vec![
        UsageEvent::for_test(Provider::Claude, "old", "2025-12-31T23:59:59Z", 99),
        UsageEvent::for_test(Provider::Claude, "session-a", "2026-01-01T00:00:00Z", 20),
        renamed,
        UsageEvent::for_test(Provider::Claude, "session-c", "2026-01-01T00:00:00Z", 7),
    ];

    let result = compute_session_aggregation(
        &events,
        "2026-01-01T00:00:11Z",
        Some("2026-01-01"),
    );

    assert_eq!(
        result.sessions.iter().map(|session| session.session_key.as_str()).collect::<Vec<_>>(),
        vec!["session-b", "session-a", "session-c"],
    );
    assert_eq!(result.sessions[0].name.as_deref(), Some("Renamed run"));
    assert_eq!(result.sessions.iter().map(|session| session.current_day_tokens).sum::<u64>(), 49);
}
```

Add a provider-summary assertion that serialized provider session totals sum to `today_tokens`, and retain the existing exact-10-second idle test.

- [ ] **Step 2: Run the focused aggregation tests to verify they fail.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test session_summary projects_current_day_sessions_active_first_with_stable_order`

Expected: compile failure because `SessionAggregate` has no session identity/name projection yet.

- [ ] **Step 3: Filter session groups to the requested day and capture the latest valid name.**

In `compute_session_aggregation`, keep grouping by `(Provider, session_key)` and keep all events for the existing aggregate semantics. When `local_day` is `Some(day)`, skip a group whose events contain no matching `timestamp_local_day`. For retained groups, calculate `current_day_tokens` from the day-filtered events, keep `active` based on the newest valid event at or before `now`, and choose the name from the newest event carrying a non-empty normalized `session_name`.

Add `session_key` and `name` to `SessionAggregate`, then sort the result with this comparator:

```rust
sessions.sort_by(|left, right| {
    right
        .active
        .cmp(&left.active)
        .then_with(|| right.last_updated_seconds.cmp(&left.last_updated_seconds))
        .then_with(|| left.session_key.cmp(&right.session_key))
});
```

Keep the existing current-session rule: sum active current-day sessions, otherwise use the newest retained session's current-day total. Preserve the 10-second strict boundary.

- [ ] **Step 4: Map aggregates into the provider wire array without duplicating accounting.**

In `compute_provider_summary`, call `compute_session_aggregation(&provider_events, now, Some(local_day))`, map each aggregate to `SessionUsageSummary`, and pass the vector to `ProviderUsageSummary::new`. Use `SessionUsageState::Active` when `session.active` is true and `SessionUsageState::Idle` otherwise. Continue using `compute_active_provider` and `compute_current_session_tokens_for_local_day` for the existing provider state/current-session behavior, including the previous-day `Some(0)` fallback.

Set the wire `id` with `safe_session_id(&session.session_key)` and pass through the already-normalized optional `session.name`. This keeps React keys stable while preventing an oversized or control-bearing internal key from crossing the Rust boundary.

- [ ] **Step 5: Run Rust aggregation and collection tests, then commit.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test session_summary`

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test provider_summary`

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test collection_core`

Expected: PASS, with old aggregate assertions unchanged and new session rows proving day filtering, names, ordering, and token conservation.

Commit:

```text
feat: project daily session usage
```

### Task 3: Parse and map the strict frontend session contract

**Files:**
- Modify: `src/lib/contracts/validation.ts`
- Modify: `src/lib/contracts/usage-summary.ts`
- Modify: `src/lib/widget-view-model.ts`
- Modify: `src/tests/lib/usage-summary.test.ts`
- Modify: `src/tests/lib/widget-view-model.test.ts`
- Modify: frontend summary fixtures in `src/tests/`

**Interfaces:**
- `SessionUsageState = "active" | "idle"`.
- `SessionUsageSummary = { id: string; name?: string; state: SessionUsageState; todayTokens: number }`.
- `ProviderUsageSummary.sessions` is required and contains zero or more validated session records.
- `WidgetSessionViewModel = { id: string; label: string; state: SessionUsageState; todayTokens: number }`.
- `WidgetProviderViewModel.sessions` is an ordered `WidgetSessionViewModel[]`; `WidgetProviderViewModel.sessionCount` is derived as `sessions.length`.

- [ ] **Step 1: Add failing parser and view-model tests.**

Extend the valid fixture with one named active session and one unnamed idle session:

```ts
const sessions = [
  { id: "run-alpha", name: "Alpha", state: "active" as const, todayTokens: 12 },
  { id: "run-beta", state: "idle" as const, todayTokens: 8 },
];
```

Assert that `parseUsageSummary` keeps both records, rejects an empty ID, rejects an unsafe token count, rejects an unknown session key, and rejects duplicate IDs within one provider. Assert that `createWidgetViewModel` returns `label: "Alpha"` for the first record and `label: "run-beta"` for the second, with `sessionCount === 2` and unchanged provider `todayTokens`.

- [ ] **Step 2: Run the focused frontend tests to verify they fail.**

Run: `npm test -- --run src/tests/lib/usage-summary.test.ts src/tests/lib/widget-view-model.test.ts`

Expected: FAIL because the current provider contract has no `sessions` field and the current view model has no session projection.

- [ ] **Step 3: Add one bounded metadata validator and strict session parsing.**

In `src/lib/contracts/validation.ts`, add the shared bound and validator used for both IDs and names:

```ts
export const MAX_SESSION_LABEL_LENGTH = 256;

export function isSafeSessionLabel(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_SESSION_LABEL_LENGTH &&
    !/[\u0000-\u001f\u007f]/u.test(value)
  );
}
```

In `usage-summary.ts`, add `sessions` to `providerSummaryKeys`, define `sessionSummaryKeys`, validate `id` with `isSafeSessionLabel`, validate optional `name` with the same helper, validate `state` against `active | idle`, validate `todayTokens` with `isSafeTokenCount`, reject duplicate IDs per provider, and return the normalized session array. Keep the provider count/order and unknown-key rejection rules unchanged. Update every existing provider fixture with `sessions: []`.

- [ ] **Step 4: Map names with ID fallback in the view model.**

Add the view-model type and map the contract without changing totals or provider visibility:

```ts
export interface WidgetSessionViewModel {
  id: string;
  label: string;
  state: "active" | "idle";
  todayTokens: number;
}

function viewForSession(session: SessionUsageSummary): WidgetSessionViewModel {
  return {
    id: session.id,
    label: session.name ?? session.id,
    state: session.state,
    todayTokens: session.todayTokens,
  };
}
```

Add `sessions: usage.sessions.map(viewForSession)` and `sessionCount: usage.sessions.length` to each visible provider view model. Keep the existing canonical provider order, preview-disabled state, aggregate metrics, and combined total calculations.

- [ ] **Step 5: Run the focused frontend tests and commit the contract change.**

Run: `npm test -- --run src/tests/lib/usage-summary.test.ts src/tests/lib/widget-view-model.test.ts`

Expected: PASS, including privacy/unknown-key rejection, name priority, ID fallback, and unchanged aggregate totals.

Commit:

```text
feat: expose sessions to widget view
```

### Task 4: Render active rows, idle disclosure, and responsive layout

**Files:**
- Create: `src/components/widget/SessionUsageList.tsx`
- Create: `src/styles/widget/sessions.module.css`
- Modify: `src/components/widget/ProviderUsageRow.tsx`
- Modify: `src/styles/widget/provider.module.css`
- Modify: `src/styles/widget/surface.module.css`
- Modify: `src/components/widget/TokenTracingWidget.tsx`
- Modify: `src/lib/widget-layout.ts`
- Modify: `src/lib/window-sizing.ts`
- Modify: `src/lib/desktop/window.ts`
- Test: `src/tests/components/widget/SessionUsageList.test.tsx`
- Test: `src/tests/components/widget/TokenTracingWidget.test.tsx`
- Test: `src/tests/lib/widget-layout.test.ts`
- Test: `src/tests/lib/window-sizing.test.ts`

**Interfaces:**
- `SessionUsageList({ sessions, onToggle }: { sessions: readonly WidgetSessionViewModel[]; onToggle?: () => void })` renders active rows directly and idle rows inside one collapsed native `<details>`.
- `syncWidgetWindowHeight(visibleProviderCount: number, measuredContentHeight?: number): Promise<void>` preserves the old fallback and clamps measured content between the provider baseline and `WIDGET_MAX_HEIGHT`.
- `widgetHeightForContent(visibleProviderCount: number, measuredContentHeight?: number): number` returns the clamped logical target height.

- [ ] **Step 1: Write failing component and layout tests.**

Create `SessionUsageList.test.tsx` with the required interaction:

```tsx
it("shows active sessions and keeps idle sessions behind a collapsed disclosure", () => {
  render(
    <SessionUsageList
      sessions={[
        { id: "active-id", label: "Active run", state: "active", todayTokens: 12 },
        { id: "idle-id", label: "Idle run", state: "idle", todayTokens: 8 },
      ]}
    />,
  );

  expect(screen.getByText("Active run")).toBeInTheDocument();
  expect(screen.getByText("12")).toBeInTheDocument();
  const disclosure = screen.getByText("Idle · 1").closest("details");
  expect(disclosure).not.toHaveAttribute("open");

  fireEvent.click(screen.getByText("Idle · 1"));
  expect(disclosure).toHaveAttribute("open");
  expect(screen.getByText("Idle run")).toBeInTheDocument();
  expect(screen.getByText("8")).toBeInTheDocument();
});
```

Add layout assertions for `widgetHeightForContent(1, undefined)`, a measured height below the one-provider baseline, a measured height above the baseline, and a value above `WIDGET_MAX_HEIGHT`. Update the existing widget test to expect the sizing call's optional measured height and the session count/rows.

- [ ] **Step 2: Run the focused frontend tests to verify they fail.**

Run: `npm test -- --run src/tests/components/widget/SessionUsageList.test.tsx src/tests/lib/widget-layout.test.ts src/tests/lib/window-sizing.test.ts`

Expected: FAIL because the session list and content-height API do not exist.

- [ ] **Step 3: Implement the native disclosure and flexible row grid.**

Create `SessionUsageList.tsx` with no React disclosure state. Define the named props and row component used by the list:

```tsx
interface SessionUsageListProps {
  sessions: readonly WidgetSessionViewModel[];
  onToggle?: () => void;
}

function SessionRow({ session }: { session: WidgetSessionViewModel }) {
  const tokens = formatTokens(session.todayTokens);
  return (
    <div className={styles.row} aria-label={`${session.label}: ${tokens} tokens`}>
      <span className={styles.label} title={session.label}>{session.label}</span>
      <strong className={styles.tokens}>{tokens}</strong>
    </div>
  );
}

export default function SessionUsageList({ sessions, onToggle }: SessionUsageListProps) {
  const active = sessions.filter((session) => session.state === "active");
  const idle = sessions.filter((session) => session.state === "idle");
  if (sessions.length === 0) return null;

  return (
    <div className={styles.list} aria-label="Today's sessions">
      {active.map((session) => <SessionRow key={session.id} session={session} />)}
      {idle.length > 0 && (
        <details className={styles.disclosure} onToggle={onToggle}>
          <summary className={styles.summary}>Idle · {idle.length}</summary>
          <div className={styles.idleRows}>
            {idle.map((session) => <SessionRow key={session.id} session={session} />)}
          </div>
        </details>
      )}
    </div>
  );
}
```

`SessionRow` must render a flexible label column and a max-content token column. Put `title={session.label}` and the resolved label in the accessible name, and format tokens with the existing `formatTokens` helper. Use CSS Modules for the list, summary focus state, row spacing, token tabular numerals, and muted idle disclosure styling. Do not slice labels in TypeScript or CSS by character count.

- [ ] **Step 4: Add the provider count and preserve memoized row behavior.**

In `ProviderUsageRow`, render `{usage.sessionCount} sessions today`, keep the existing `Session`/`Today` aggregate metrics, and append `SessionUsageList`. Extend `areProviderUsageRowsEqual` to compare `sessionCount` and every session's `id`, `label`, `state`, and `todayTokens`. Keep provider identity/status comparison unchanged.

Add the count style to `provider.module.css`. Change `.providerList` to `overflow: auto` with `min-height: 0` so content beyond the native maximum remains reachable. Keep the existing shadow-free/widget surface treatment and provider row spacing.

- [ ] **Step 5: Make window height follow disclosed content without polling.**

Add `widgetHeightForContent` to `widget-layout.ts`:

```ts
export function widgetHeightForContent(
  visibleProviderCount: number,
  measuredContentHeight?: number,
): number {
  const minimum = widgetHeightForVisibleProviders(visibleProviderCount);
  const measuredHeight = measuredContentHeight;
  if (measuredHeight === undefined || !Number.isFinite(measuredHeight)) return minimum;
  return Math.max(
    minimum,
    Math.min(WIDGET_MAX_HEIGHT, Math.ceil(measuredHeight)),
  );
}
```

In `TokenTracingWidget`, keep refs for the root and provider list, increment a local layout revision from `SessionUsageList.onToggle`, and measure the total desired content as the provider list's `scrollHeight` plus the current root height outside that list. Call `syncWidgetWindowHeight(viewModel.visibleProviderCount, measuredContentHeight)` from an effect keyed by the view model and layout revision. If the DOM is unavailable in a test or measured height is zero, pass `undefined` and retain the existing provider-count baseline.

Update the desktop bridge to use `widgetHeightForContent`, keep the existing resize queue/latest-request guard, set `maxHeight: WIDGET_MAX_HEIGHT`, and leave width clamping at 360-720. The list scrolls when measured content exceeds 520.

- [ ] **Step 6: Run widget tests and commit the UI change.**

Run: `npm test -- --run src/tests/components/widget src/tests/lib/widget-layout.test.ts src/tests/lib/window-sizing.test.ts`

Expected: PASS, including keyboard/native disclosure behavior, active-first rendering, ID/name labels, token columns, baseline/max height behavior, and existing drag/resize behavior.

Commit:

```text
feat: render daily session rows
```

### Task 5: Complete cross-boundary regression coverage and verification

**Files:**
- Modify: every remaining Rust constructor/test call site found by `rg -n "ProviderUsageSummary::new" src-tauri/src src-tauri/tests`
- Modify: `src-tauri/tests/provider_readers.rs` only if the optional observation field changes its expected struct comparison
- Modify: `src/tests/lib/usage-summary.test.ts` and all summary fixtures that construct provider records
- Modify: `src/tests/components/widget/TokenTracingWidget.test.tsx` and `src/tests/components/widget/ProviderSection.test.tsx` for the final required provider child structure

**Interfaces:**
- No new command, event, adapter result, or frontend filesystem path is introduced.
- Existing `get_usage_summary` and `usage-summary-changed` payloads validate and serialize the same top-level fields plus provider `sessions` arrays.

- [ ] **Step 1: Add rename and serialization regression assertions.**

Use two events with the same session key and different valid names, apply them in timestamp order, query the summary, and assert the final name is the newer one while the ID and token totals remain unchanged. Add a direct serialization assertion that a session contains only `id`, `name` when present, `state`, and `todayTokens`, and that no source path/raw field appears anywhere in the payload.

- [ ] **Step 2: Run all required automated gates.**

Run:

```text
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug
```

Expected: every command exits successfully with no privacy-test failure, contract mismatch, formatting change, or generated artifact staged.

- [ ] **Step 3: Perform the Windows smoke check.**

In the debug Tauri app, verify both providers render their current-day count, active rows are visible, idle rows are collapsed until keyboard/mouse disclosure, each row shows a token value, names fall back to IDs, and resizing from the minimum toward the maximum width changes available label space without changing the token column. Verify a crowded session list remains reachable via the bounded internal scroll area and no source path or raw record appears.

- [ ] **Step 4: Review the final diff and commit the verification updates.**

Run `git status --short`, `git diff --check`, and `git diff --stat`; confirm only the spec, plan, Rust source/tests, TypeScript source/tests, and focused CSS are present. Keep build output, profiles, dependency directories, and local settings untracked/ignored.

Commit any final test-only adjustments with:

```text
test: verify multi-session display
```

## Plan self-review

- Spec coverage: Tasks 1-2 cover Rust identity, mutable names, migration, day filtering, active/idle state, ordering, and token conservation; Tasks 3-4 cover strict transport parsing, name fallback, disclosure, responsive width, and bounded height; Task 5 covers privacy, rename, full gates, and Windows smoke behavior.
- Placeholder scan: no unspecified implementation step is used; every code change names its file, interface, test, command, and expected result.
- Type consistency: `SessionUsageState`/`SessionUsageSummary` are defined in Task 1, consumed by Rust projection in Task 2, mirrored as `SessionUsageSummary` in Task 3, and mapped to `WidgetSessionViewModel` before Task 4 consumes it. The optional `measuredContentHeight` parameter is introduced once and used consistently by the layout and desktop bridge.
