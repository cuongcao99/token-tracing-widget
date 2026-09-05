# Signed application updates implementation plan

## Goal

Implement the approved signed updater slice on `feat/app-updates` with the
smallest complete change. Keep the existing Rust/React settings boundary,
reuse the generic SQLite settings table, and commit each cohesive purpose
separately with its tests.

## Atomic implementation sequence

1. Persist `UpdateSettingsSnapshot { auto_update: boolean }` through the Rust
   settings store, typed commands, TypeScript contract parser, and existing
   settings persistence queue.
2. Add the Rust-owned Tauri updater service, signed updater configuration,
   manual check/install commands, and the one-shot startup automatic-update
   path.
3. Add the Settings Updates section, manual action states, automatic-update
   switch, and focused frontend behavior tests.
4. Update the GitHub release workflow to publish signed updater artifacts and
   stable `latest.json` metadata; document the signing secrets and SemVer
   release requirement.

## Verification

Run focused tests after each slice and stage only its intended files. Finish
with the repository's frontend tests/build, Rust format/check/tests, debug
Tauri build, and a Windows signed-release smoke check when credentials and a
test release are available.

## Constraints

Do not change provider adapters, collection scheduling, SQLite schema, tray
behavior, window geometry, or the frontend dependency set. Do not add generic
updater abstractions, polling, retries, telemetry, or unrelated cleanup.
