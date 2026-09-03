# Codex session day/index source update

Date: 2026-09-02
Status: Accepted implementation correction

## Decision

For Codex, daily session admission follows the native source metadata before a
session file is read:

1. Its UUID must match any entry in `.codex/session_index.jsonl`.
2. Only then is the file read for validated token observations and optional
   `thread_name` metadata.

The dated folder is the session's starting-day storage location and must not
prevent an indexed session from being read when it receives a later-day
append.

The index controls admission, while `updated_at` only selects the latest
display name and the dated folder is only the session's storage location. Token
totals remain derived from the existing validated, deduplicated usage-event
pipeline. When a
previous run stored Codex events from files that fail the new admission rule,
the current-day summary excludes those rows without deleting the SQLite
ledger or source files.

This narrows the earlier multi-session display rule that used accepted event
timestamps alone for Codex file admission. The existing event timestamp rule
still determines current-day token totals after admission. The default runtime
keeps all discovered file metadata available; the reader enforces a 50 MiB
per-pass byte budget and resumes each physical file from its persisted
checkpoint on the next pass.

An unindexed session remains excluded from current-day collection even when its
file contains a current-day event.
