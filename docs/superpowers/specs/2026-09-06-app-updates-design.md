# Signed application updates design update

Date: 2026-09-06

Status: Approved implementation slice

## Decision

Add an opt-in application update flow to the Settings window. The flow has two
user-facing controls:

- `Check for updates`, which performs one manual check and reports whether a
  newer version is available;
- `Automatic updates`, which persists a boolean preference and enables one
  update check during the next application startup.

When automatic updates are enabled and a newer signed version is found, the
application downloads, installs, and restarts. Manual installation remains an
explicit user action. There is no polling timer, retry loop, update history,
rollback control, release channel selector, or background service in this
slice.

## Trust and privacy boundary

The updater is Rust-owned. React can call only typed Tauri commands and receives
safe update metadata, such as the current and available versions. It does not
use a network updater package and never receives update URLs, signatures, raw
network errors, or installer contents.

The only new network access is a fixed HTTPS endpoint for signed Tauri updater
metadata and artifacts hosted by this project's GitHub Releases. No provider
data, source paths, credentials, repository contents, usage events, or
telemetry are sent to that endpoint.

Tauri's updater signature verification remains enabled. The public signing key
is committed in Tauri configuration; the private signing key exists only in
GitHub Actions secrets.

## Settings and persistence

Automatic updates are stored as `update.auto_update` in the existing generic
SQLite settings table. Missing or invalid values default to `false`. No schema
migration is required.

The setting is loaded through a typed command and saved immediately through the
existing serialized settings persistence queue. Closing Settings continues to
wait for pending persistence.

## Runtime behavior

The Rust update service owns check/install operations and shares one in-flight
operation guard between startup and manual actions. A startup check is
non-blocking and never prevents live collection from starting. A failed
automatic check is non-fatal and is not surfaced as a collection error.

The manual Settings UI shows only idle, checking, up-to-date, available,
installing, and friendly error states. The install action rechecks availability
before downloading so stale check results cannot install an older or already
replaced release.

## Release compatibility

The release workflow must publish signed updater artifacts and `latest.json`.
The `latest` endpoint requires a stable GitHub Release, so the current
prerelease-only publish setting changes to a stable release. Each production
release must also increase the application SemVer; the feature implementation
does not perform a separate version bump.

## Out of scope

- silent update checks before the user opts in;
- periodic update polling;
- cloud sync, telemetry, provider changes, or collection changes;
- update channels, downgrade support, rollback, or release-note rendering;
- code signing beyond the Tauri updater artifact signature;
- a JavaScript updater state library or generic network abstraction.
