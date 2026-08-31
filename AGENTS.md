# Repository Working Agreement

## Source-of-truth documents

- Read `CONTEXT.md` before introducing or changing domain terminology.
- Read `PRODUCT.md` before changing product scope, user-facing behavior,
  or the version-one boundary.
- Read `design/DESIGN_APPLE.md` for the Apple visual reference, design tokens,
  and typography guidance.
- Read `design/DESIGN_CLAUDE.md` for the Claude-editorial visual direction used
  by the current overlay and settings presentation.
- Read `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`
  before changing architecture, privacy boundaries, or version-one scope.
- Read the relevant dated design-update spec and plan before changing an
  already-approved presentation slice. Record an explicit design update when
  departing from an approved decision.

## Product boundaries

- Keep version one local-only and Windows 11-only.
- Keep filesystem, collection, source discovery, and SQLite access in Rust.
  The React webview receives typed summaries, plus configured source roots only
  in settings flows.
- Preserve metadata-only collection: prompts, responses, reasoning, tool
  payloads, credentials, repository contents, working directories, raw
  provider records, and arbitrary file contents never enter normalized events,
  SQLite, diagnostics, or frontend payloads.
- Keep provider-specific formats behind adapters. Enforce normalization, delta
  conversion, deduplication, validation, and checkpoint invariants in the Rust
  collection core.
- Keep settings preview and persistence semantics intact: an edit may preview
  immediately across the widget and settings windows, while saving is required
  to persist it and closing an unsaved edit restores the saved snapshot.
- Add no network client, telemetry, sidecar, background service, frontend state
  library, CSS framework, ORM, or font package without an approved design
  change.

## Repository map

- `src-tauri/src/app/` owns runtime startup, live collection orchestration,
  window lifecycle, and tray actions.
- `src-tauri/src/providers/` owns Claude Code and Codex adapters, readers, and
  record parsers.
- `src-tauri/src/sources/` owns bounded source discovery, source configuration,
  and file watching.
- `src-tauri/src/collection/`, `src-tauri/src/usage/`,
  `src-tauri/src/database/`, `src-tauri/src/commands/`, and
  `src-tauri/src/types/` own collection invariants, aggregate calculations,
  SQLite access, typed Tauri commands, and frontend-facing contracts
  respectively.
- `src/components/widget/`, `src/components/settings/`, and
  `src/components/shared/` contain focused React components. Cross-component
  orchestration belongs in `src/hooks/`; typed bridges and pure transforms
  belong in `src/lib/`; visual tokens and layout belong in `src/styles/`.
- `src/tests/` contains frontend tests separated from implementation folders;
  `src-tauri/tests/` contains Rust integration and contract tests.
- `design-preview.html` and `src/design-preview.{css,js}` are the static visual
  review surface. They do not replace the runtime React/Tauri surfaces.
- `PRODUCT.md` contains product scope and brand commitments. `design/` contains
  the maintained visual references. Dated implementation specs and plans live
  under `docs/superpowers/`.

## UI and window rules

- Keep the widget and settings surfaces split into focused components and
  responsibility-specific styles; avoid returning to monolithic view files.
- Use the existing typed Tauri bridge for usage summaries, settings, preview
  events, native dragging, and native resize handles. Do not implement window
  movement or resizing with a JavaScript polling loop.
- Preserve the current frameless, transparent, taskbar-hidden, non-topmost
  utility-window behavior, the shared six-dot drag affordance, native resize
  handles, responsive widget height, and the approved surface treatment
  (settings elevation with a shadow-free widget) unless a new design decision
  explicitly changes them.
- Resolve typography through the documented system-font stack. Do not bundle
  or download a font as part of the product.

## Development and verification

- Use `dev` for ongoing work. Treat `main` as finalized-only: merge or push
  code there only after the relevant frontend, Rust, integration, and privacy
  checks pass.
- Work test-first for behavior changes and add the narrowest regression proof
  at the responsible layer.
- For frontend changes, run `npm test -- --run` and `npm run build`.
- For Rust changes, run `cargo fmt --manifest-path src-tauri/Cargo.toml
  -- --check`, `cargo check --manifest-path src-tauri/Cargo.toml`, and
  `cargo test --manifest-path src-tauri/Cargo.toml`.
- For cross-boundary changes, also run `npm run tauri build -- --debug` and
  perform the relevant Windows smoke checks.
- Stage only intended source, documentation, and test changes. Keep generated
  build output, browser-review artifacts and profiles under `.impeccable/`,
  dependency directories, and local `.claude/` settings out of commits; only
  intentionally shared Impeccable `config.json` files may be committed.
- Keep changes scoped to the requested behavior; do not opportunistically
  refactor unrelated modules.
