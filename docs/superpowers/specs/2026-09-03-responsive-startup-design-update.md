# Responsive Startup and Usage Loading

Date: 2026-09-03
Status: approved implementation update

## Decision

Tauri setup must return without running the first collection synchronously.
After the tray and `AppState` are initialized, it starts the existing live
collection worker and returns. The worker's existing 200 ms notification
debounce triggers the first collection and publishes the normal summary.

The widget renders a frontend-only loading skeleton while the typed usage
summary is in `loading`. The existing `active`, `idle`, `unavailable`, and
`stale` states keep their current presentation.

## Preserved boundaries

- No IPC or normalized usage contract changes.
- Rust remains responsible for discovery, file reads, collection, and SQLite.
- Metadata-only collection and the 50 MiB per-attempt content-read budget stay
  unchanged.
- No change to the unlimited discovery limit used for multi-day history.

## Rationale

The synchronous setup collection performs recursive source discovery and reads
provider files on the native startup path. On a cold local data set it can
hold the native event loop long enough for Windows to show `(Not Responding)`.
Deferring that same work to the already-owned worker keeps the window
responsive without duplicating the collection path.

The skeleton is intentionally structural rather than animated shimmer: it
reuses the widget's semantic color tokens, exposes `aria-busy`, and disables
its pulse under reduced motion. Each visible provider keeps the real provider
section geometry, including its heading, limit slots, metric slots, and
updated-time slot. Session rows are omitted until the real summary arrives.
The small `ProviderLoadingSkeleton` layout contract keeps those slot counts
reusable when another provider or metric set is added.
