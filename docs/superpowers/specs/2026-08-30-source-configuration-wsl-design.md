# Source Configuration and WSL Root Support Design

**Date:** 2026-08-30  
**Status:** Proposed for review  
**Scope:** Rust collection core, local SQLite settings, root resolution, and live watcher refresh

## Context

The current runtime always enables Claude Code and Codex using their automatic
native Windows roots. The collection core already has an `enabled` field, but
the value is hard-coded and there is no persisted source-preference accessor.
The watcher is created once and only reconciled on its timer, so a source-root
change cannot take effect immediately.

The approved product design already allows each provider to be enabled
independently and allows Claude Code to use an explicitly selected WSL UNC
folder. This slice makes that engine behavior real while keeping the settings
surface and visual work for later.

## Goals

- Load and persist per-provider enabled state and optional root overrides.
- Preserve automatic native roots as the default and preserve existing
  databases without a schema migration.
- Accept explicit local Windows directories and the approved WSL UNC shape
  (`\\wsl.localhost\\<distribution>\\...`) without invoking `wsl.exe` or
  enumerating distributions.
- Validate roots independently so one bad source does not disable the other.
- Use the effective configured root for discovery, source health, and the
  `sources` table instead of overwriting it with a native-only relative label.
- Reload collection and watcher roots after a configuration change through a
  path-free internal signal.
- Exclude disabled providers from active-provider and today's-total
  calculations while retaining their historical normalized events.
- Keep raw provider records, conversational content, credentials, repository
  contents, and working-directory data out of normalized events, diagnostics,
  and frontend payloads.

## Non-goals and explicit deferrals

- No React settings screen or visual redesign in this slice.
- No always-on-top/always-below decision or window behavior change.
- No startup registration, single-instance work, clear-index flow, or
  database backup/rebuild flow.
- No network client, WSL command execution, distro enumeration, sidecar, or
  background service.
- No arbitrary filesystem browsing API exposed to the webview.

The next surface slice will add a minimal settings window and its typed Tauri
boundary on top of this engine. The final UI slice will handle visual polish
and the desktop placement behavior requested by the user.

## Design options considered

### A. Reuse the existing SQLite `settings` key/value table — selected

Store preferences under stable provider-scoped keys:

```text
source.claude.enabled       = 0 | 1
source.claude.root_override = absolute UTF-8 path, or key absent
source.codex.enabled        = 0 | 1
source.codex.root_override  = absolute UTF-8 path, or key absent
```

An absent key means the default: enabled and automatic native root. Removing a
root-override key returns that provider to automatic discovery. The existing
`sources` table remains the health-and-effective-configuration mirror written
by collection; it is not a second source of truth for preferences.

This avoids a schema migration, preserves the approved storage boundary, and
keeps source preferences separate from transient health.

### B. Add a dedicated source-configuration table — rejected for this slice

It would provide a more strongly typed schema, but would duplicate the
existing `sources` row and require a migration before the behavior is useful.
The current key/value table is sufficient for two fixed providers and the
settings contract can validate values at the Rust boundary.

### C. Add a JSON configuration file — rejected

This would create a second local persistence mechanism beside SQLite and would
make the source configuration/health relationship harder to recover and test.

## Domain model

Add a Rust-private source configuration model conceptually equivalent to:

```text
SourceConfig
  provider: Provider
  enabled: bool
  root_override: Option<PathBuf>

SourceConfigSet
  claude: SourceConfig
  codex: SourceConfig

RootSelection
  AutomaticNative
  Explicit(PathBuf)

ResolvedSourceRoot
  provider
  filesystem_path
  configured_root_label
  origin: automatic_native | explicit
```

`SourceConfig` is the persisted user choice. `ResolvedSourceRoot` is an
ephemeral validated path used by discovery and the watcher. The effective
label is:

- `.claude/projects` or `.codex/sessions` for an automatic native root;
- the explicit configured path for an override.

The explicit path is allowed in the local `settings` table and the local
`sources.configured_root` mirror because collection requires it. It is never
put in a normalized usage event, diagnostic message, overlay summary, or
generic frontend payload.

Defaults are applied independently per provider:

- `enabled = true` when the enabled key is absent or invalid;
- `root_override = None` when the override key is absent or invalid.

Malformed preference values do not prevent startup and are not silently
deleted. The next collection records only a bounded category such as
`invalid_settings`; it does not record the bad value or path in diagnostics.

## Persistence contract

`database::settings` owns typed accessors for the four stable keys. It exposes
operations equivalent to:

- load both provider configurations with independent defaulting;
- save one provider configuration transactionally;
- remove an override key when switching back to automatic discovery.

The enabled value is encoded as `0` or `1`. The root override is stored as the
UTF-8 path string supplied by the internal settings API. The settings accessor
must reject empty strings, NUL-containing strings, and values that fail root
syntax validation before writing them.

Configuration updates follow this order:

1. Validate and normalize the requested provider configuration in Rust.
2. Write the preference keys in one SQLite transaction.
3. Update the in-memory `SourceConfigSet` only after the write succeeds.
4. Ask the live collector to refresh its watcher roots and schedule a
   debounced collection.

If the database write fails, the in-memory configuration and watcher remain
unchanged and the caller receives a sanitized storage error. If the refresh
signal is lost during shutdown, the persisted configuration is still loaded on
the next startup and the normal reconciliation pass repairs the watcher.

## Root validation and resolution

### Automatic native roots

Automatic selection uses the existing fixed provider-relative paths below the
current Windows user profile:

```text
Claude Code: .claude/projects
Codex:      .codex/sessions
```

The existing safe-path checks remain mandatory: no parent traversal, absolute
path injection into a profile-relative join, or reparse-point traversal. A
missing native root becomes `not_detected`; a blocked or unsafe root becomes
its corresponding sanitized health state.

### Explicit roots

An explicit root may be either:

1. an absolute local Windows directory, including a drive-qualified path; or
2. a WSL UNC path whose server is exactly `wsl.localhost` (case-insensitive),
   followed by a non-empty distribution component and an optional absolute
   path within that distribution, for example:
   `\\wsl.localhost\\Ubuntu\\home\\user\\.claude\\projects`.

Reject relative paths, URI syntax, device paths, arbitrary network shares,
empty distribution names, `.`/`..` traversal, NUL characters, and any other
UNC server. The resolver never calls `wsl.exe`, talks to a distro, or discovers
available distros.

The user may save a syntactically safe explicit root even when it is currently
missing. This supports a stopped/unmounted WSL distribution. Collection then
reports `not_detected` until the directory appears. If a selected path exists,
it must be a directory and must pass the existing reparse-point safety rules;
the walker must validate every discovered child remains beneath that root.

Only existing validated directories become `WatchRoot` entries. WSL UNC roots
use the same Windows directory-change watcher when the OS accepts it. If the
watcher cannot arm, it emits the existing sanitized unavailable signal and the
30-second reconciliation scan remains authoritative.

## Discovery and collection changes

Provider-specific readers remain unchanged. The source-discovery layer gains a
configured-root entry point alongside the native convenience function. It
returns a `DiscoveryResult` with a runtime-owned configured-root label rather
than a `&'static str` native-relative path.

The runtime builds one `ProviderSource` per known provider from the loaded
`SourceConfigSet`:

- disabled sources do not resolve, walk, or read files;
- enabled sources resolve their automatic or explicit root independently;
- a root failure produces an empty result for that provider and leaves the
  other provider collectable.

`ProviderSource` carries the effective configured-root label so collection
writes `SourceUpdate.configured_root` from configuration, not from discovery's
native-only path field. Disabled sources write health state `disabled`, while
enabled missing/blocked/invalid sources retain their existing sanitized states.

The source update still contains only provider, enabled state, health category,
configured-root label, and timestamp. It does not contain file paths beyond the
explicit configured root already permitted by the local source-settings
boundary, and it never contains record data.

## Enabled-source aggregation

Disabling a source stops future discovery, reading, and watching. It does not
delete historical events or checkpoints. Summary computation receives the
current enabled-provider set and filters events before selecting the active
provider, current-session total, and today's total.

Re-enabling a source makes its retained normalized events eligible again. This
preserves restart-safe history without silently destroying data when a user
temporarily disables a provider.

The summary continues to expose a sanitized `SourceHealth` entry for each
known provider. A disabled entry uses the state `disabled`; the overlay still
receives no root path or raw source details.

## Live reload and watcher lifecycle

Extend the internal watcher signal with a path-free
`ConfigurationChanged` variant. The live collector handle exposes an internal
refresh method that sends this signal after a successful settings write.

On `ConfigurationChanged`, the live loop:

1. asks the shared `AppState` for the current effective watch roots;
2. replaces watcher workers atomically from the loop's point of view;
3. marks the scheduler changed so collection runs after the normal debounce;
4. keeps the existing retry and 30-second reconciliation behavior.

The signal contains no path. The watcher still emits only provider-level
signals, so file names and raw filesystem events cannot cross the collection
boundary.

## Error and recovery behavior

- Invalid settings for Claude do not change Codex configuration.
- Missing explicit roots are valid persisted choices and report
  `not_detected` until available.
- Permission/invalid/reparse failures are mapped to sanitized root categories.
- A watcher failure does not stop reconciliation or the other provider.
- A failed settings write leaves the previous config active.
- A collection/storage failure preserves the existing stale-summary and
  bounded-retry behavior.
- No error path logs the configured path, source record, prompt, response,
  reasoning, tool payload, credential, repository content, or working
  directory.

## Testing strategy

### Settings and persistence

- absent keys load enabled + automatic defaults;
- valid enabled and override values round-trip through SQLite;
- removing an override returns to automatic selection;
- malformed values default independently and produce only a sanitized
  category;
- failed writes do not mutate in-memory configuration.

### Root validation

- native roots retain profile-boundary and reparse checks;
- absolute local roots are accepted/rejected according to the explicit rules;
- `\\wsl.localhost\\Ubuntu\\...` is accepted syntactically;
- arbitrary UNC, device, URI, relative, traversal, and empty-distro paths are
  rejected;
- missing but syntactically safe roots can be saved and later resolve as
  `not_detected`;
- existing roots and discovered children remain within the validated root.

### Collection and aggregation

- configured-root labels survive source updates for native and explicit roots;
- disabled sources do not read or watch files;
- disabled-provider events are excluded from summaries and return when
  re-enabled;
- source health remains independent across providers;
- native and WSL-style temporary roots exercise the same adapter/discovery
  path.

### Live collector

- `ConfigurationChanged` replaces watcher roots and schedules one debounced
  collection;
- the signal carries no path and does not alter existing shutdown/retry
  behavior;
- refresh after a failed persistence operation is not sent.

Existing collection, privacy-contract, frontend-build, Rust, and integrated
Tauri gates remain required before this slice is considered complete.

## Acceptance criteria

This slice is complete when:

1. Both providers load independent persisted enabled/root settings with safe
   defaults from the existing SQLite database.
2. Automatic native roots and explicit local/approved WSL UNC roots share one
   validated discovery path.
3. A missing explicit root can be persisted and later becomes collectable when
   it appears, without restarting the app.
4. A successful configuration update changes watcher roots and schedules a
   collection without exposing a path through the overlay contract.
5. Disabled providers are not read or watched, and their historical events
   are excluded from summaries without being deleted.
6. Native and explicit configured-root labels are not overwritten by the
   collection health update.
7. All new behavior has narrow Rust regression tests plus the relevant
   existing frontend/Rust/integration/privacy gates.

