# Repository Working Agreement

## Context

- Read `CONTEXT.md` before introducing or changing domain terminology.
- Read `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md` before changing product behavior, architecture, privacy boundaries, or version-one scope.
- Treat the approved design as authoritative; record an explicit design update before departing from it.

## Product boundaries

- Keep version one local-only and Windows 11-only.
- Keep filesystem, collection, and SQLite access in Rust. The React webview receives typed summaries, plus configured source roots only in settings flows.
- Preserve metadata-only collection: prompts, responses, reasoning, tool payloads, credentials, repository contents, and working directories never enter normalized events, SQLite, diagnostics, or frontend payloads.
- Keep provider-specific formats behind adapters and enforce normalization, delta conversion, deduplication, and checkpoint invariants in the collection core.
- Add no network client, telemetry, sidecar, background service, frontend state library, CSS framework, or ORM without an approved design change.

## Development workflow

- Use `dev` for ongoing work. Treat `main` as finalized-only: merge or push code there only after the relevant frontend, Rust, integration, and privacy checks pass.
- Work test-first for behavior changes and add the narrowest regression proof at the responsible layer.
- Run the relevant scripts declared in `package.json` and checks for `src-tauri/Cargo.toml` before completion. For cross-boundary changes, verify both frontend and Rust plus an integrated Tauri build.
- Keep changes scoped. Do not commit local `.claude/` settings unless the user explicitly requests them.
