# Concurrent Windows and WSL Source Design

**Date:** 2026-09-04  
**Status:** Approved for implementation on `codex/wsl-windows-multi-root`  
**Supersedes:** The single explicit-root behavior in
`docs/superpowers/specs/2026-08-30-source-configuration-wsl-design.md`

## Decision

Each registered Provider can collect from its Windows source and one optional
WSL source at the same time. Windows remains the default automatic source;
WSL is an explicitly selected additional source. The existing provider-level
collection switch stays the single enable/disable control for all configured
roots of that Provider.

This supports a user running native Windows and WSL sessions concurrently
without introducing WSL command execution, distro discovery, a helper process,
or a second collection pipeline.

## Settings experience

The Sources row keeps the existing collection switch and replaces the inline
root-path action with `Change source`. Activating it opens an accessible dialog
for the selected Provider with two independent options:

- **Windows** — automatic `%USERPROFILE%` source by default, or a selected
  local Windows folder. The current path and a `Change Windows source` action
  are shown.
- **WSL** — not configured by default, or a selected
  `\\wsl.localhost\\<distribution>\\...` folder. The current path and a
  `Choose WSL source`/`Change WSL source` action are shown. A configured WSL
  root can be removed.

The dialog states that Windows and WSL can both be active. The dialog itself
does not expose a radio selection: choosing WSL must not turn off Windows.
The native Windows folder picker remains the only filesystem-selection UI.

### Design update — editable source roots

The source rows use an editable text input for each root, with a folder icon
button beside it for the native folder picker. Empty inputs show a
platform-specific example as a placeholder; configured values use normal
ink text. Blur or Enter commits typed values through the existing typed Rust
command, while an empty value clears the optional override.

## Typed model and persistence

The Rust source configuration becomes:

```text
SourceConfig
  provider: Provider
  enabled: bool
  windows_root_override: Option<PathBuf>
  wsl_root_override: Option<PathBuf>
```

`windows_root_override = None` means the fixed native Windows root. A WSL
override is absent until the user chooses one; a configured WSL path remains a
valid persisted choice even when its distribution is stopped or unavailable.

The settings boundary exposes:

```text
SourceSettings
  provider: "claude" | "codex"
  enabled: boolean
  windowsRoot: string | null
  wslRoot: string | null
```

The existing `settings` key/value table remains the persistence mechanism. New
keys are `source.<provider>.windows_root_override` and
`source.<provider>.wsl_root_override`. Existing `root_override` values are
read once as compatibility data: WSL-shaped values become the WSL override;
other valid values become the Windows override. Successful writes remove the
legacy key for that Provider.

## Validation and source resolution

Windows overrides accept absolute local Windows paths only. WSL overrides
accept only an absolute UNC path whose server is exactly `wsl.localhost`
(case-insensitive), with a non-empty distribution component. Relative paths,
URI syntax, device paths, parent traversal, arbitrary network shares, and
cross-platform placement are rejected before persistence.

The resolver always creates the Windows source entry and creates a WSL source
entry only when a WSL override exists. Missing explicit roots remain persisted
and report `not_detected`; existing roots must pass the existing reparse-point
and containment checks. Discovery walks every resolved entry with the existing
provider adapter and metadata-only limits.

The collection core processes all root discoveries for one Provider in one
collection batch. File identities already include the Provider and full
filesystem path hash, so equivalent Windows and WSL session filenames cannot
share a checkpoint accidentally. Provider health is aggregated to one summary
entry: a usable root keeps the Provider usable when another configured root is
missing or unavailable. Source health remains independent between Providers.

The live observer owns multiple workers per Provider and emits the existing
provider-only signals. A configuration refresh replaces all workers for the
affected effective root set. No filesystem path, root kind, raw notification,
or provider record crosses the observer/React boundary.

## Error and privacy behavior

Picker cancellation leaves all settings unchanged. Picker and update failures
remain sanitized to the existing error categories. Source paths may appear in
the Settings source-editing flow and local settings/effective-root storage,
but never in normalized events, diagnostics, summaries, observer signals, or
the overlay.

The feature remains Windows 11-only, local-only, metadata-only, and uses the
existing Rust-owned filesystem/SQLite boundary. It does not invoke `wsl.exe`,
enumerate WSL distributions, add a network client, or add a sidecar/service.

## Acceptance criteria

1. A Provider with only default Windows configuration behaves exactly as
   before.
2. A Provider can have Windows and WSL roots configured simultaneously, and
   new events from both roots contribute to one Provider and aggregate total.
3. Existing single-root settings load without data loss and map to the right
   platform.
4. Settings visibly offers `Change source` and an accessible Windows/WSL
   dialog for both Claude Code and Codex.
5. Selecting or removing WSL refreshes collection/watchers without restarting
   the app; a missing WSL root recovers when it appears.
6. One broken root does not suppress a usable root or the other Provider.
7. Frontend contract, Rust, privacy, build, and packaged Windows checks pass.

## Explicitly out of scope

- automatic WSL distro discovery or `wsl.exe` execution;
- a WSL-side helper, IPC protocol, service, or Linux/macOS desktop build;
- per-root health in the overlay or raw session/path display outside Settings;
- more than one explicitly configured WSL root per Provider in this slice;
- changes to provider adapters, token normalization, delta conversion, or
  summary semantics.
