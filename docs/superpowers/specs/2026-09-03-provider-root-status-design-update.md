# Provider Root and User-Facing Status Design Update

Date: 2026-09-03
Status: Accepted implementation direction

## Source boundary

Settings and the live observer treat the provider directory as the source
root:

- Claude Code: `.claude`
- Codex: `.codex`

Collection remains scoped to the provider's session directory:

- Claude Code: `.claude/projects`
- Codex: `.codex/sessions`

This lets the Codex observer notice changes to the sibling
`.codex/session_index.jsonl` without scanning unrelated provider files such as
configuration, history, or credentials. The observer still emits only a
provider-level signal.

## User-facing status vocabulary

The settings surface collapses internal discovery and reader states into a
small vocabulary:

- Source health: `Available`, `Unavailable`, or `Off`.
- Provider activity: `Active`, `Idle`, `Unavailable`, or transient `Checking`.

The Rust collector keeps its detailed states for diagnostics and recovery.
`detected`, `limited`, and `malformed` are usable source states and display as
`Available`; missing, invalid, inaccessible, or unavailable roots display as
`Unavailable`; disabled sources display as `Off`.
