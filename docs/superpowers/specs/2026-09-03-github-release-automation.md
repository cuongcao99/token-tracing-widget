# GitHub Release Automation Design

## Goal

Validate changes on the development path and publish a Windows installer when
changes reach `main`.

## Decisions

- CI runs on pull requests targeting `main` and on pushes to `dev`.
- Release runs on every push to `main`, which covers a merged `dev` pull
  request and keeps the workflow compatible with protected-branch settings.
- The release job runs on `windows-latest`, builds the Tauri NSIS installer,
  and creates a GitHub prerelease through `tauri-apps/tauri-action@v0`.
- Release tags use `v<tauri-version>-build.<workflow-run-number>`, for example
  `v0.1.0-build.42`. This gives every main build a unique immutable tag while
  the product version remains `0.1.0` until a deliberate version bump.
- The release job uses the built-in `GITHUB_TOKEN` with `contents: write`; no
  repository secret is needed for this unsigned first release pipeline.
- The Tauri bundle is enabled for the `nsis` target only, producing the Windows
  setup executable rather than unrelated platform packages.

## Non-goals

- Code signing and certificate management.
- Tauri updater metadata and automatic in-app updates.
- Multi-platform builds.
- Automatic semantic-version calculation or commits back to `main`.

## Acceptance criteria

- A pull request into `main` runs frontend and Rust verification on Windows.
- A push to `dev` runs the same verification without publishing anything.
- A push to `main` verifies first, then creates one uniquely tagged GitHub
  prerelease containing the Windows NSIS installer.
- A failed verification prevents the release job from running.
- The workflow requests only read access for CI and write access for publishing.
