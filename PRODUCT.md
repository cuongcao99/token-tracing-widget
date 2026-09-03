# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

The primary user is a developer who runs Claude Code or Codex on Windows 11 and
wants a quick, glanceable view of local token usage while continuing to work in
other applications.

## Product Purpose

Token Tracing derives privacy-safe token-usage totals from supported local
coding-agent session data and presents the latest provider, current-session
total, today's total, and last update in a small desktop overlay. Success means
the user can understand current usage without opening session files.

## Positioning

The product combines local-first collection with a metadata-only boundary:
provider records are normalized into token metadata without retaining prompts,
responses, reasoning, tool payloads, credentials, repository contents, or
working directories.

## Operating Context

The overlay runs as one Tauri 2 application with a Rust-owned collector and
SQLite index, a React/TypeScript webview, and a system tray. It observes Claude
Code and Codex sources from automatic native roots or approved explicit WSL
Claude Code roots. The overlay is intentionally compact and non-intrusive so
it can remain visible while the user works elsewhere.

## Capabilities and Constraints

- Version one is Windows 11-only, local-only, and metadata-only.
- Rust owns filesystem, collection, and SQLite access; React receives typed
  summaries and settings payloads only.
- Claude Code and Codex sources can be enabled independently, with independent
  source health and restart-safe totals.
- Enabled source roots are observed by the native file observer while the app
  is open; Active expires after 15 seconds without a newer valid token event.
- The runtime uses Tauri commands and events, plain CSS, SQLite, and no network
  client, telemetry, sidecar, background service, frontend state library, CSS
  framework, ORM, or WSL auto-discovery.
- The overlay remains approximately 320 by 120 logical pixels, frameless,
  transparent, taskbar-hidden, and draggable from its non-interactive header.
- The approved placement update changes the overlay from always-on-top to a
  normal non-topmost window; it remains interactive and closing it hides it.
- The widget must preserve readable active, idle, loading, unavailable, and
  stale states without exposing raw source data or absolute source paths.

## Brand Commitments

- The product name is Token Tracing Widget.
- The user-provided `design/DESIGN_APPLE.md` is the visual authority for the redesign:
  Apple-inspired restraint, system/SF-style typography, Action Blue as the
  single interactive accent, near-black ink, light/parchment surfaces,
  hairlines instead of heavy chrome, no decorative gradients, and restrained
  elevation.
- The redesign should feel modern and native-like without copying Apple's
  product-specific imagery, logo, navigation, or marketing composition.

## Evidence on Hand

- `CONTEXT.md` defines the product vocabulary and domain model.
- `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md` is the
  approved architecture and version-one source of truth.
- `design/DESIGN_APPLE.md` contains the user-provided Apple UI/UX analysis and token
  references.
- The current repository contains the Rust collection/runtime, React overlay,
  source settings window, and tray flow.
- No product photography, logo asset, customer proof, or marketing claims are
  available or required for the overlay redesign.

## Product Principles

- Privacy is visible through restraint and never competes with the usage read.
- The overlay should be understandable in one glance while the user is busy.
- Native platform behavior and reliable data matter more than decorative novelty.
- Provider failures remain independent and recoverable.

## Accessibility & Inclusion

- Maintain readable contrast for all states and visible keyboard focus for any
  interactive controls introduced by the redesign.
- Respect `prefers-reduced-motion`; motion must never be required to understand
  a state or total.
- Keep the header drag affordance visually clear while preserving a non-drag
  path for any future interactive controls.
