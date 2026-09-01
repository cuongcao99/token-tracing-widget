# Agent hooks and no-terminal Windows distribution

**Date:** 2026-09-01  
**Scope:** Research only; no application code changed.  
**Question:** Can the Windows 11 Tauri app be downloaded and run by ordinary users while agent-installed hooks replace filesystem logs as the lifecycle signal source?

## Executive conclusion

The feasible product shape is **signed Windows installer + first-run, user-consented hook setup + local metadata-only IPC**. The installer can be a per-user NSIS `-setup.exe` published as a GitHub Release asset; users need neither Git nor a terminal to download, install, and launch it. Tauri documents both NSIS setup executables and MSI packages, and its default Windows install mode is current-user-only, which avoids administrator privileges and installs under `%LOCALAPPDATA%` ([Tauri Windows Installer](https://v2.tauri.app/distribute/windows-installer/)).

Hooks are suitable for **lifecycle state**, not for replacing the usage-data source. The current repository defines token totals from provider session data and deliberately keeps raw records inside Rust ([approved widget design](../superpowers/specs/2026-08-29-token-tracing-widget-design.md); [native provider formats](../compatibility/2026-08-29-native-provider-formats.md)). The documented Claude Code and Codex hook contracts contain prompt/assistant content and identifiers, but do not define token-count fields ([Claude hooks reference](https://code.claude.com/docs/en/hooks); [Codex hooks](https://developers.openai.com/codex/hooks)). Therefore the recommended migration is:

```text
agent hook -> start/continue/pause/stop lifecycle signal
provider session data -> metadata-only token observations and totals
```

This replaces filesystem activity as the **state-transition trigger**, while retaining a bounded filesystem reader (or another future provider-approved usage API) for token accounting. Removing filesystem reads entirely would make the current token-total feature unsupported by the documented hook APIs.

## Claude Code

### Supported events relevant to tracing

Claude Code documents `UserPromptSubmit` as firing when the user submits a prompt, before Claude processes it. Its input includes common fields such as `session_id`, `prompt_id` (on sufficiently recent versions), `transcript_path`, `cwd`, `permission_mode`, and `hook_event_name`, plus the submitted `prompt` text ([common input fields](https://code.claude.com/docs/en/hooks#common-input-fields); [UserPromptSubmit](https://code.claude.com/docs/en/hooks#userpromptsubmit)). This is a good `start_or_continue` transition, but the hook process must discard `prompt`, `transcript_path`, `cwd`, and any other non-allow-listed value.

`Stop` runs after the main Claude agent has finished responding. It does **not** run when the stoppage is caused by a user interrupt; API errors use `StopFailure` instead ([Stop](https://code.claude.com/docs/en/hooks#stop)). The `Stop` input includes `stop_hook_active`, `last_assistant_message`, `background_tasks`, and `session_crons` in addition to common fields ([Stop input](https://code.claude.com/docs/en/hooks#stop-input)). This makes `Stop` useful as a turn-level `pause` signal, but it is not an exact “user pressed End” event.

`SessionEnd` is the stronger session-level end signal. Claude documents reasons including `clear`, `resume`, `logout`, `prompt_input_exit`, and `other`; it has no decision control and is intended for cleanup or logging ([SessionEnd](https://code.claude.com/docs/en/hooks#sessionend)). Treat it as `stop` for the session, while treating `Stop` as `pause` for an individual completed turn. A user interrupt that does not terminate the session remains an observable gap; the app should retain an inactivity/heartbeat fallback rather than claim exact end detection.

### Payload, exit, and timeout behavior

Command hooks receive JSON on stdin and communicate with exit codes, stdout, and stderr ([Claude hook execution](https://code.claude.com/docs/en/hooks#how-hooks-work)). For most events, exit code `2` blocks the operation; specifically, it blocks prompt processing for `UserPromptSubmit` and prevents Claude from stopping for `Stop` ([exit code behavior](https://code.claude.com/docs/en/hooks#exit-code-2-behavior-per-event)). A lifecycle observer must always exit `0`, emit no stdout, and emit no stderr. This prevents it from injecting context, blocking a prompt, or keeping Claude working.

Claude cancels a synchronous command hook at its timeout and discards its output; `UserPromptSubmit` has a 30-second default for command/http/MCP hooks, while `SessionEnd` has a 1.5-second shared budget that can be raised by configuration up to 60 seconds ([timeouts](https://code.claude.com/docs/en/hooks#timeouts); [SessionEnd timeout](https://code.claude.com/docs/en/hooks#sessionend-input)). The receiver should do only bounded stdin parsing and a local non-blocking signal attempt, with a short explicit timeout. An asynchronous command hook avoids delaying Claude, but its result is delivered only while the session is alive and it can be canceled during teardown ([async hooks](https://code.claude.com/docs/en/hooks#run-hooks-in-the-background)); synchronous short hooks are safer for the final `stop` signal.

Claude warns that command hooks execute with the full user permissions and can access or modify anything the user can access ([security considerations](https://code.claude.com/docs/en/hooks#security-considerations)). The installer and first-run UI must make this explicit, show the exact installed executable identity, provide disable/remove controls, and never install a hook silently.

### Scope and installation locations

Claude hooks are configured in JSON settings files. The documented scopes are user-wide `~/.claude/settings.json`, project-shareable `.claude/settings.json`, project-local `.claude/settings.local.json`, managed policy settings, and plugin `hooks/hooks.json` ([hook locations](https://code.claude.com/docs/en/hooks#hook-locations)). For this product, the least invasive public-user choice is the user scope, because it does not write into repositories. The app must merge only its own handler into the existing JSON, preserve unrelated hooks, avoid a standalone `hooks.json` file (Claude says there is no standalone hooks file), and provide a reversible uninstall path ([configuration troubleshooting](https://code.claude.com/docs/en/debug-your-config)).

There is no documented Claude “install this third-party hook” API in the reviewed sources. The practical implementation is therefore an app-owned edit of the user settings JSON after explicit consent. A plugin could package hooks, but it adds a separate plugin distribution/trust surface and is not needed for a first-party desktop app.

## Codex

### Public hook support and useful events

Codex now has a documented public hooks surface. It supports turn events including `UserPromptSubmit` and `Stop`, session-start events including `SessionStart`, and main-thread termination through `SessionEnd` ([Codex event table](https://developers.openai.com/codex/hooks#when-hooks-run)). `UserPromptSubmit` includes `session_id`, `turn_id`, and the prompt text; `Stop` includes `turn_id`, `stop_hook_active`, and the latest assistant message; `SessionEnd` includes a reason that is currently always `other` ([UserPromptSubmit](https://developers.openai.com/codex/hooks#userpromptsubmit); [Stop](https://developers.openai.com/codex/hooks#stop); [SessionEnd](https://developers.openai.com/codex/hooks#sessionend)).

The lifecycle mapping can therefore mirror Claude:

| Provider event | Normalized signal | Meaning |
| --- | --- | --- |
| `UserPromptSubmit` | `start_or_continue` | A new user turn is about to be processed |
| `Stop` | `pause` | The current turn has stopped/finished |
| `SessionEnd` | `stop` | The main session ended |

Codex documents that `SessionEnd` may run when a conversation is closed, archived/deleted while open, normally closed, or idle for 30 minutes; switching away does not end it immediately ([Codex SessionEnd lifecycle](https://developers.openai.com/codex/hooks#sessionend)). This is not a reliable immediate “window closed” notification. Do not depend on an undocumented `Interrupt` or desktop-specific event: the official release docs are the release-behavior reference and explicitly warn that schemas on the repository `main` branch may include fields not in the current release ([Codex schemas note](https://developers.openai.com/codex/hooks#schemas)).

### Payload, trust, and Windows limits

Codex discovers hooks from `~/.codex/hooks.json`, `~/.codex/config.toml`, project `.codex/hooks.json`, project `.codex/config.toml`, and enabled plugins. It merges matching sources rather than replacing lower-precedence hooks ([Codex locations](https://developers.openai.com/codex/hooks#where-codex-looks-for-hooks)). A Windows-specific `commandWindows` override is documented, and command/MCP handlers are supported while prompt/agent handlers are parsed but skipped ([Codex config shape](https://developers.openai.com/codex/hooks#config-shape)).

Non-managed hooks must be reviewed and trusted before they run; Codex records trust against the current hook-definition hash and skips new or changed definitions until trusted. The documented one-off bypass is `--dangerously-bypass-hook-trust` ([Codex review and trust](https://developers.openai.com/codex/hooks#review-and-trust-hooks)). The app must not use that bypass. This is the main obstacle to a completely no-terminal public onboarding: the reviewed public docs expose `/hooks` as the review/trust UI, but do not expose a third-party API for the desktop app to mark a user hook trusted. The first-run UI should show `installed`, `awaiting Codex trust`, `active`, or `fallback`, rather than pretending installation is complete.

Codex command hooks can run synchronously or with `async: true`; background hooks cannot block or control the operation, and `SessionEnd` is always synchronous. Codex documents a one-second default and three-second maximum for `SessionEnd` ([Codex async behavior and limits](https://developers.openai.com/codex/hooks#run-hooks-in-the-background); [Codex configuration notes](https://developers.openai.com/codex/hooks#config-shape)). Use no stdout/stderr and exit `0` for the observer. Never return `additionalContext`, `decision`, `continue: false`, or any other control output.

As with Claude, the documented Codex hook payload is not a token-usage API. The contract lists session/turn identifiers, working directory, model and permission metadata, plus prompt or assistant text for the relevant events; it does not define normalized input/output token counts ([Codex common input fields](https://developers.openai.com/codex/hooks#common-input-fields)). Hooks can safely produce a metadata-only signal only if the receiver deliberately projects an allow-list and keeps the raw stdin in process memory only; the provider itself still sends sensitive fields to the hook process.

## Metadata-only signal design

The hook executable should be a small hook mode of the signed installed application, not a second service or a Node/Python sidecar:

```text
Claude/Codex command hook
  -> installed Token Tracing executable --hook <provider>
  -> parse only event/session/turn identifiers and bounded timestamps
  -> send TraceSignal over a local authenticated named pipe or equivalent IPC
  -> exit 0 with empty stdout/stderr
```

The main Tauri process owns the IPC listener and existing Rust collection state. If the app is not running, the hook should fail open and exit quickly; it must not start the UI, block the agent, or write a raw fallback log. A conceptual signal is:

```text
TraceSignal {
  schema_version,
  provider,
  lifecycle: start_or_continue | pause | stop,
  opaque_session_id?,
  opaque_turn_id?,
  provider_event,
  observed_at,
  sequence?
}
```

The receiver must discard prompt text, assistant text, transcript paths, working directories, tool inputs/outputs, error details, model text, and unknown fields before IPC or persistence. It should reject oversized input, malformed JSON, unexpected event names, and identifiers that exceed bounded limits. The privacy tests should feed synthetic payloads containing recognizable forbidden values and assert that neither the emitted signal, SQLite, diagnostics, frontend payloads, nor stdout/stderr contains them.

This boundary is **not automatically safe** merely because the final signal is small: the provider hook process receives the original JSON first. The design is safe only if the installed binary is trusted, the receiver is fail-closed for parsing and fail-open for provider behavior, the IPC endpoint is local and access-controlled, and no raw input is logged or returned.

## Public Windows distribution

### Recommended first public channel

1. Build signed x64 Windows artifacts in GitHub Actions. Tauri documents `tauri-action` for building and uploading installers to a GitHub Release ([Tauri GitHub pipeline](https://v2.tauri.app/distribute/pipelines/github/)).
2. Publish the NSIS `-setup.exe` as the primary download. Tauri documents NSIS setup executables and MSI as the two Windows installer forms ([Tauri Windows Installer](https://v2.tauri.app/distribute/windows-installer/)). Use MSI as an additional enterprise/deployment artifact if needed.
3. Keep the default per-user install mode initially. Tauri documents that it avoids administrator privileges and installs under `%LOCALAPPDATA%`; `perMachine` requires elevation ([install modes](https://v2.tauri.app/distribute/windows-installer/#install-modes)). This matches hook configuration under the current user profile.
4. Link the release page and a stable direct asset URL from the project website/readme. GitHub documents both `/releases/latest` and `/releases/latest/download/<asset>` links ([GitHub release links](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases)). A user journey is therefore: browser download -> run setup.exe -> complete installer -> launch from Start Menu/tray -> approve hook setup in the app.

The current repo has bundling disabled in [`src-tauri/tauri.conf.json`](../../src-tauri/tauri.conf.json), so a future packaging change must explicitly enable the chosen bundle target and add installer smoke coverage. This research does not make that change.

### WebView2 choice

Windows 11 distributes WebView2 as part of the operating system, according to Tauri’s installer guide. The default `downloadBootstrapper` path keeps the installer small but can require internet access if the runtime is missing; Tauri documents an `offlineInstaller` option that adds about 127 MB and a fixed runtime option that adds about 180 MB ([WebView2 installation options](https://v2.tauri.app/distribute/windows-installer/#webview2-installation-options)). For a normal Windows 11 public download, retain the system-managed/default path. Offer an offline artifact only for a separately defined offline deployment requirement.

### Signing and update tradeoffs

Sign the installer and shipped executable using one stable publisher identity. Tauri documents Windows signing through an OV certificate, Azure Key Vault, or a custom signing command ([Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/)). Microsoft’s current SmartScreen guidance says a valid certificate can still show an “unrecognized app” warning until publisher/file reputation accumulates, and that unsigned files can be blocked by Smart App Control on Windows 11 ([Microsoft SmartScreen reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation)). Do not promise that EV alone removes all warnings: an older statement on the Tauri page conflicts with Microsoft’s current guidance, which says EV no longer bypasses SmartScreen by default.

Start with manual GitHub Release downloads and defer in-app updates until the release pipeline and signing identity are stable. Tauri’s updater requires signed update artifacts; it cannot be disabled, needs a public key in configuration and a protected private key for builds, and uses HTTPS endpoints in production ([Tauri updater signing](https://v2.tauri.app/plugin/updater/)). Enabling it would also introduce a networked update path into a repository whose approved V1 boundary is local-only. If an app-store channel becomes important later, Tauri documents a Microsoft Store path, but it requires an offline WebView2 installer, silent installation, auto-update handling, and code signing ([Tauri Microsoft Store](https://v2.tauri.app/distribute/microsoft-store/)).

## Recommended architecture and staged rollout

### Preserve the current ownership boundary

Keep Rust as the owner of provider adapters, validation, deduplication, usage deltas, SQLite, diagnostics, and typed summaries. Add a Rust-owned `TraceSignal` ingress beside the existing collection coordinator. The signal updates lifecycle state and schedules/reconciles collection; it does not carry or cause frontend access to raw provider records. This preserves the repository’s approved data flow and privacy constraints ([approved architecture](../superpowers/specs/2026-08-29-token-tracing-widget-design.md)).

### Rollout

1. **Packaging baseline:** enable a signed per-user NSIS release, publish it as a GitHub Release asset, verify clean install/upgrade/uninstall, and record SmartScreen behavior. No hook behavior changes yet.
2. **Hook contract and receiver:** add a versioned metadata-only signal contract and local IPC design; test malformed/oversized/sensitive payloads and app-not-running behavior with synthetic inputs.
3. **Claude native Windows:** first-run consent writes a user-scope Claude hook for `UserPromptSubmit`, `Stop`, `StopFailure`, and `SessionEnd`, with short bounded command execution. Validate normal completion, user interrupt, API failure, `/clear`, resume, disabled hooks, existing user hooks, and upgrade/uninstall.
4. **Codex native Windows:** write user-scope `~/.codex/hooks.json` using `commandWindows`; show the Codex trust state and require the provider’s supported review flow. Validate interactive CLI and any supported desktop surface separately; do not treat repository `main` source as a substitute for a released contract.
5. **Dual-path accounting:** use hook lifecycle signals as the fast trigger and keep the existing bounded filesystem reader as the usage-accounting/reconciliation path. Compare hook-driven state against the 30-second reconciliation path and prove no double counting.
6. **Fallback and only then cleanup:** if a provider is absent, hooks are disabled/untrusted, the provider version lacks an event, or WSL invocation is not proven, expose an explicit degraded state and retain the bounded reader/manual state fallback. Do not remove filesystem usage accounting until a supported replacement for token counts exists and passes privacy/restart/rotation tests.

## Risks and open decisions

- **Hook payload exposure:** both providers send sensitive content to command hooks. The product must explain this consent and prove raw-field non-retention.
- **Codex trust UX:** public documentation does not provide a third-party auto-trust API. Decide whether asking users to open `/hooks` is acceptable, or whether Codex should remain fallback-only for consumer onboarding.
- **End semantics:** `Stop` is turn completion, not necessarily session termination; Claude omits it on user interrupt, and Codex `SessionEnd` can be delayed. Define whether the widget’s “pause/stop” is turn-scoped or session-scoped.
- **WSL Claude:** the reviewed sources do not establish that a Windows-installed executable can be invoked reliably as a hook from every WSL configuration. Validate this before claiming parity with the repository’s existing explicit WSL-root support.
- **Codex release drift:** the docs warn that repository schemas may be ahead of the current release. Maintain a provider-version compatibility matrix and a startup capability check.
- **Hook config conflicts:** existing user/project/managed/plugin hooks can coexist, block, or be disabled by policy. Merge surgically and provide a visible health state.
- **Signing cost and reputation:** signing reduces trust friction but does not guarantee zero SmartScreen warnings for a new publisher/file. Choose the publisher identity and CI key custody before public release.
- **Updater/network boundary:** decide whether the product remains manual-download/local-only for V1 or approves a signed HTTPS updater as a later architectural change.

## Source gaps

The reviewed official sources do not provide a metadata-only hook payload mode, a generic third-party hook installation/trust API, a stable Codex desktop hook contract, or a guarantee that a Windows executable hook works from WSL. They also do not expose token counts through lifecycle hooks. These are implementation-validation items, not assumptions to encode as product guarantees.

