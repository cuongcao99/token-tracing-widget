<div align="center">

# ✦ Token Tracing Widget

**A quiet, glanceable token-usage overlay for your coding agents.**

![Windows 11](https://img.shields.io/badge/Windows%2011-0078D4?style=flat-square&logo=windows11&logoColor=white)
![Tauri 2](https://img.shields.io/badge/Tauri%202-FFC131?style=flat-square&logo=tauri&logoColor=111111)
![Rust](https://img.shields.io/badge/Rust-DEA584?style=flat-square&logo=rust&logoColor=111111)
![React and TypeScript](https://img.shields.io/badge/React%20%2B%20TypeScript-61DAFB?style=flat-square&logo=react&logoColor=111111)
![Local only](https://img.shields.io/badge/Local--only-2E7D32?style=flat-square&logo=shield&logoColor=white)

</div>

Token Tracing Widget is a local-first Windows 11 desktop overlay for viewing
privacy-safe token usage from **Claude Code** and **Codex**.

It reads supported provider session data locally, normalizes token metadata,
keeps restart-safe totals in a local SQLite index, and presents a compact
summary while you work. The widget shows provider activity, current-session
usage, today's usage, rate limits when available, and the latest update time.

## ✦ Product boundaries

- 🪟 Windows 11 only, local only, and metadata only.
- 🤝 Claude Code and Codex are supported independently.
- 🔒 Prompts, responses, reasoning, tool payloads, credentials, repository
  contents, and raw provider records never enter the normalized data path.
- ⚙️ The app uses one Tauri executable with a Rust native core and a React/
  TypeScript webview.
- 🌱 No cloud sync, telemetry, remote API, sidecar, or provider configuration
  changes are required.

## ✦ Documentation

- [Product scope and commitments](PRODUCT.md)
- [Domain vocabulary and current product state](CONTEXT.md)
- [System architecture](ARCHITECTURE.md)
- [Architecture diagram](architecture/01-collect-to-ui.html)
- [Visual references](design/)
- [Implementation records and research](docs/)

The README intentionally stays at product level. Technical ownership,
runtime flow, privacy seams, and extension guidance live in
[`ARCHITECTURE.md`](ARCHITECTURE.md) and the linked source-of-truth documents.

## ✦ Release notes

### Unreleased · `0.1.0`

- ✨ Live token summaries for Claude Code and Codex.
- ⚡ Non-blocking startup with provider-aware loading skeletons.
- 🎞️ Rolling-number transitions when token totals change.
- 🧱 Local SQLite persistence with typed Rust/Tauri and React boundaries.
- 📚 Architecture diagrams and technical documentation consolidated under
  `architecture/` and [`ARCHITECTURE.md`](ARCHITECTURE.md).
