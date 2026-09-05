# Architecture

Token Tracing Widget is a Windows-first Tauri 2 application with a Rust-owned
collection and storage core and a React/TypeScript presentation layer. The
architecture keeps provider-specific formats behind adapters and exposes only
validated, metadata-only summaries to the webview.

## Source of truth

The repository vocabulary and product boundary are defined by
[`CONTEXT.md`](CONTEXT.md) and [`PRODUCT.md`](PRODUCT.md). The approved
architecture decision is recorded in
[`docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`](docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md).
Dated design updates under `docs/superpowers/specs/` record later decisions;
the implementation and current context remain authoritative when an older
plan describes a superseded flow.

## System view

The primary data path is shown in the maintained
[collection-to-widget diagram](architecture/01-collect-to-ui.html). Its
[Archify JSON source](architecture/01-collect-to-ui.architecture.json) is the
editable source; regenerate the HTML viewer from that source instead of
editing the generated artifact by hand.

```text
local provider files
        |
        v
source discovery and observer
        |
        v
provider adapters and bounded readers
        |
        v
validated observations -> deltas -> deduplicated usage events
        |
        v
SQLite index -> summary aggregation
        |
        v
typed Tauri commands/events -> React widget
```

The observer emits provider-level signals. It does not send file paths,
filenames, or raw records to the frontend. A signal schedules the existing
collection path; discovery and reading then happen inside Rust.

## Ownership map

| Area | Location | Owns |
| --- | --- | --- |
| App shell | `src-tauri/src/app/` | Startup, live collection orchestration, window lifecycle, tray actions, and native folder picking |
| Sources | `src-tauri/src/sources/` | Provider roots, configuration, bounded session-file discovery, and the Windows file observer |
| Provider adapters | `src-tauri/src/providers/` | Claude Code and Codex readers, parsers, rate-limit extraction, session metadata, and the shared adapter contract |
| Collection core | `src-tauri/src/collection/` | Per-source collection, validation handoff, delta conversion, deduplication handoff, checkpoints, and transactional batch composition |
| Usage calculation | `src-tauri/src/usage/` | Cumulative-to-delta conversion, duplicate identity rules, provider/session/day aggregation, and active-provider selection |
| Storage | `src-tauri/src/database/` | SQLite schema, normalized events, sessions, checkpoints, source health, settings, rate limits, and sanitized diagnostics |
| Tauri boundary | `src-tauri/src/commands/` and `src-tauri/src/types/` | Command handlers, event names, serialization contracts, and frontend-facing summaries |
| Application updates | `src-tauri/src/app/updates.rs` and `src-tauri/src/commands/updates.rs` | Rust-owned signed update checks/install operations and sanitized update metadata |
| Widget UI | `src/components/widget/`, `src/hooks/`, `src/lib/`, `src/styles/` | Summary subscription, view-model derivation, loading/active/error states, layout, animation, and rendering |
| Settings UI | `src/components/settings/`, `src/settings-main.tsx` | Source, widget, and update preferences through typed settings/update commands and preview events |
| Tests | `src/tests/` and `src-tauri/tests/` | Frontend behavior/contracts and Rust unit, integration, storage, provider, and privacy-boundary coverage |

Each area should expose the smallest seam needed by its consumer. Filesystem,
provider data, and SQLite access stay behind Rust-owned boundaries; React
receives typed summaries and settings payloads only.

## Runtime flow

1. Tauri setup creates the tray and initializes `AppState` from the current
   Windows profile and local database path.
2. Setup starts the live collection worker and returns without running a
   recursive source scan on the native startup path.
3. If automatic updates are enabled, setup schedules one non-blocking signed
   update check/install operation. This path does not block the live worker.
4. The worker refreshes observers for enabled provider roots and schedules the
   initial collection. File changes are coalesced by the live scheduler, with
   reconciliation and retry behavior kept in the same worker.
5. `AppState` resolves source configuration. The runtime discovers the
   provider session directories beneath the configured roots and selects the
   registered adapter for each provider.
6. An adapter reads bounded record data from its provider files starting at a
   persisted checkpoint. It emits `ProviderReadResult` values containing
   normalized observations, positions, and provider-specific metadata such as
   rate limits.
7. The collection core validates observations, converts cumulative counters
   to deltas, filters duplicates, updates session metadata, and composes a
   `CollectionBatch`.
8. `IndexStore::apply_batch` commits events, sessions, rate limits, source
   health, diagnostics, and checkpoints in one SQLite transaction.
9. The coordinator queries committed rows and computes provider, session, and
   current-day totals. The active provider is derived from the newest valid
   token event.
10. The typed `UsageSummary` is published through the
   `usage-summary-changed` event. The `get_usage_summary` command remains the
   initial read path and fallback when event setup is unavailable.
11. React validates the wire payload, maps it into a widget view model, and
    renders the widget without polling SQLite or provider files.

## Boundaries and invariants

### Privacy boundary

Only normalized token metadata and bounded status values may leave the Rust
collection core. The following never enter SQLite, diagnostics, IPC payloads,
or the React layer:

- prompts, responses, reasoning, and tool payloads;
- credentials, repository contents, and working directories;
- raw provider records and arbitrary file contents; and
- absolute source paths, filenames, or provider-specific opaque records.

Configured source-root overrides are the deliberate exception in settings
flows because the user needs to review and replace them. They are not part of
the usage summary sent to the widget.

The updater is a separate deliberate exception: Rust may contact only the
configured HTTPS signed-release endpoint and may return only safe version
metadata. Provider data, source paths, credentials, usage events, and raw
network or installer content remain outside the updater boundary.

### Provider seam

`ProviderAdapter` is the shared internal contract. Claude Code and Codex own
their parsing rules, while the collection core owns validation, delta
semantics, deduplication, checkpoint progression, and summary composition.
Unknown record kinds are ignored; malformed or incomplete input degrades the
affected provider without crashing the application.

### Storage seam

`CollectionStore` is the collection-facing persistence seam. The SQLite
`IndexStore` is its current implementation. The React layer has no database
dependency, and collection code does not depend on SQLite-specific queries.

### Frontend seam

The Rust `UsageSummary` contract is the only usage payload consumed by the
widget. The TypeScript contract parser rejects unknown or unsafe wire shapes
before view-model construction. Tauri commands and events are defined in the
desktop bridge modules and remain the only native communication path.

### Settings flow

Settings changes preview immediately through typed events and then persist
through typed commands. Source configuration updates refresh the live observer
after persistence. Update preferences persist through the same serialized
settings queue; update checks and installation use separate typed commands.
Widget visibility, source health, presentation state, and update state remain
separate from token accounting.

## Extension points

### Adding a provider

The smallest complete provider change is:

1. Add the provider identity and display metadata to the Rust and TypeScript
   registries.
2. Implement the existing `ProviderAdapter` contract in a provider module.
3. Add parser/reader fixtures and provider-specific tests.
4. Extend only the summary or settings presentation where the new provider
   actually needs different behavior.

Do not add a provider-specific branch to SQLite or the generic widget when the
existing normalized contract already covers the behavior.

### Adding a metric

Metrics should follow the existing direction from source metadata to summary:

```text
provider record
  -> normalized observation
  -> validated usage event or derived aggregate
  -> UsageSummary/provider summary
  -> widget view model
  -> metric component
```

First decide whether a metric is cumulative, incremental, rate-limit metadata,
or a derived value. Persist it only when restart-safe history or checkpointed
reconstruction requires persistence. Then extend the narrowest contract and
add one provider-independent test at each changed boundary.

The current provider summary already groups provider totals, session totals,
and rate limits. This gives future input/output/cache metrics a stable place
without creating a second collection pipeline or a provider-specific UI model.

### Abstraction rule

Prefer a deep, stable seam over a layer of wrappers. A new trait, registry, or
configuration object should be introduced only when it removes a real
cross-module dependency or has a second concrete implementation. Keep simple
one-use transformations local to their owning module.

## Failure and recovery

- Providers are discovered, read, and reported independently.
- Missing, inaccessible, invalid, limited, or malformed sources become
  provider health states and sanitized diagnostics.
- Partial final records remain pending for a later append.
- File truncation or rotation can restart from a safe checkpoint and rely on
  event identity deduplication.
- SQLite write failures do not publish an uncommitted summary; the live worker
  retains the last known summary and applies bounded retry behavior.
- The widget distinguishes loading, active, idle, unavailable, and stale
  states without exposing raw source data.

## Verification

The repository's normal gates are:

```text
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Cross-boundary changes also require a debug Tauri build and the relevant
Windows smoke checks. See `AGENTS.md` for the working agreement and the dated
plans/specs for the acceptance criteria of individual slices.
