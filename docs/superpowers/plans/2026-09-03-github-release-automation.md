# GitHub Release Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Validate the Windows Tauri application on pull requests and `dev`, then publish a uniquely tagged NSIS installer for every push to `main`.

**Architecture:** Keep CI and release as two small GitHub Actions workflows. CI owns repeatable verification; release repeats that gate in a `verify` job and makes the publish job depend on it. Tauri owns bundling and `tauri-action` owns GitHub Release creation, so no custom release script or release service is added.

**Tech Stack:** GitHub Actions, `windows-latest`, Node/npm, Rust stable, Tauri 2, `tauri-apps/tauri-action@v0`.

**Spec:** `docs/superpowers/specs/2026-09-03-github-release-automation.md`

## Global Constraints

- Keep version one Windows 11-only, local-only, and metadata-only.
- Keep filesystem, collection, source discovery, and SQLite access in Rust.
- Do not add a network client, telemetry, sidecar, frontend state library, CSS framework, ORM, or font package.
- Use `dev` for ongoing work and treat `main` as finalized-only.
- Request `contents: read` for verification and `contents: write` only for the publish job.
- Do not add code signing, updater configuration, automatic version bumps, or multi-platform matrices.

---

### Task 1: Add the CI verification workflow

**Files:**

- Create: `.github/workflows/ci.yml`

**Interfaces:**

- Consumes: root `package-lock.json`, `src-tauri/Cargo.toml`, and the existing npm/Rust verification commands.
- Produces: a required Windows verification workflow for pull requests into `main` and pushes to `dev`.

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/ci.yml` with `pull_request` targeting `main`, `push` to `dev`, and `workflow_dispatch` triggers. Use `windows-latest`, `actions/checkout@v4`, `actions/setup-node@v4` with npm caching, `dtolnay/rust-toolchain@stable`, and `npm ci`.

Run these existing gates in order:

```yaml
- name: Run frontend tests
  run: npm test -- --run
- name: Build frontend
  run: npm run build
- name: Check Rust formatting
  run: cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
- name: Check Rust
  run: cargo check --manifest-path src-tauri/Cargo.toml
- name: Test Rust
  run: cargo test --manifest-path src-tauri/Cargo.toml
```

Set `contents: read` and cancel superseded runs for the same workflow/ref.

- [ ] **Step 2: Validate the workflow shape**

Run `git diff --check` and inspect the workflow so the triggers, runner, permissions, and command paths match the repository layout.

- [ ] **Step 3: Commit**

```text
git add .github/workflows/ci.yml
git commit -m "ci: verify frontend and Rust on Windows"
```

### Task 2: Enable the Windows installer bundle

**Files:**

- Modify: `src-tauri/tauri.conf.json:36-39`

**Interfaces:**

- Consumes: the existing `icons/icon.ico` asset.
- Produces: an enabled NSIS bundle for the release workflow.

- [ ] **Step 1: Update the bundle config**

Change the existing bundle block to:

```json
"bundle": {
  "active": true,
  "targets": ["nsis"],
  "icon": ["icons/icon.ico"]
}
```

Do not add updater, signing, WebView, or other platform settings.

- [ ] **Step 2: Validate the config locally**

Run `npm run tauri build -- --debug` and verify that a Windows `.exe` installer is produced under `src-tauri/target/debug/bundle/nsis/`.

- [ ] **Step 3: Commit**

```text
git add src-tauri/tauri.conf.json
git commit -m "build(tauri): enable Windows NSIS bundle"
```

### Task 3: Add the main-branch release workflow

**Files:**

- Create: `.github/workflows/release.yml`

**Interfaces:**

- Consumes: the CI command set from Task 1, the NSIS bundle from Task 2, the Tauri version in `src-tauri/tauri.conf.json`, and the built-in `GITHUB_TOKEN`.
- Produces: a uniquely tagged GitHub prerelease with the Windows installer for each push to `main`.

- [ ] **Step 1: Add the gated workflow**

Create a workflow triggered by `push` to `main` and `workflow_dispatch`. Add `concurrency` with `cancel-in-progress: false` so two main releases cannot race.

The `verify` job repeats the Windows commands from Task 1. The `publish` job uses `needs: verify`, checks out the full history, installs Node/Rust dependencies, and runs:

```yaml
- uses: tauri-apps/tauri-action@v0
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  with:
    tagName: v__VERSION__-build.${{ github.run_number }}
    releaseName: Token Tracing Widget v__VERSION__ build ${{ github.run_number }}
    releaseBody: Automated Windows build for `${{ github.sha }}`.
    releaseDraft: false
    prerelease: true
    generateReleaseNotes: true
```

Give `verify` `contents: read` and `publish` `contents: write`. Do not add a
certificate secret or a write-capable token to the verification job.

- [ ] **Step 2: Inspect the release contract**

Confirm that the tag contains the configured Tauri version plus the unique run number, the publish job is gated by `needs: verify`, and the workflow is Windows-only.

- [ ] **Step 3: Commit**

```text
git add .github/workflows/release.yml
git commit -m "ci: publish Windows builds from main"
```

### Task 4: Run the repository gates and hand off the first release

**Files:**

- Verify: `.github/workflows/ci.yml`
- Verify: `.github/workflows/release.yml`
- Verify: `src-tauri/tauri.conf.json`

**Interfaces:**

- Consumes: all workflow and bundle changes from Tasks 1-3.
- Produces: verified commits on `dev`, ready for a `dev → main` merge to exercise the release pipeline.

- [ ] **Step 1: Run frontend gates**

Run `npm test -- --run` and `npm run build`.

- [ ] **Step 2: Run Rust gates**

Run `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml`.

- [ ] **Step 3: Run the Tauri debug bundle smoke check**

Run `npm run tauri build -- --debug` and verify the NSIS installer path exists. Do not commit generated `dist/` or `src-tauri/target/` output.

- [ ] **Step 4: Review the final diff**

Run `git diff --check`, `git status --short`, and inspect the staged paths. Preserve the pre-existing line-ending-only status on `src/styles/widget/surface.module.css` unless it develops a real content diff.

- [ ] **Step 5: Commit and push**

Push the workflow/config commits to `origin/dev`. The first release is exercised only when the branch is merged into `main`; no manual release tag is created from `dev`.
