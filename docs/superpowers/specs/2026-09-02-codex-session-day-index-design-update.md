# Codex session day/index source update

Date: 2026-09-02
Status: Accepted implementation correction

## Decision

For Codex, daily session admission follows the native source layout before a
session file is read:

1. The file must be under `.codex/sessions/YYYY/MM/DD` for the requested local
   day.
2. Its UUID must match an entry in `.codex/session_index.jsonl` whose
   `updated_at` belongs to that local day.
3. Only then is the file read for validated token observations and optional
   `thread_name` metadata.

The index and dated folder are admission metadata only. Token totals remain
derived from the existing validated, deduplicated usage-event pipeline. When a
previous run stored Codex events from files that fail the new admission rule,
the current-day summary excludes those rows without deleting the SQLite
ledger or source files.

This narrows the earlier multi-session display rule that used accepted event
timestamps alone for Codex file admission. The existing event timestamp rule
still determines current-day token totals after admission.
