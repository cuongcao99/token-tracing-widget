# Bounded Native Source Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Discover bounded, safe Claude Code and Codex session files beneath their fixed native Windows roots and return only sanitized provider-relative metadata to the Rust collection boundary.

**Architecture:** provider_roots.rs maps each supported Provider to one fixed relative root beneath an explicit profile directory. safe_paths.rs performs lexical containment checks and rejects symlink/reparse-point components before session_files.rs walks the tree, applies file/byte limits, and creates opaque internal file handles whose public metadata contains no absolute path, source identifier, timestamp, or file content. Claude and Codex are discovered through separate calls and their results are returned independently.

**Tech Stack:** Rust 2021, Tauri 2 workspace, std::fs, std::path, tempfile integration fixtures, Windows reparse-point metadata, existing Provider type

**Spec:** docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md

**Handoff:** C:/Users/caocu/AppData/Local/Temp/token-tracing-widget-handoff-2026-08-29.md

## Global Constraints

- Keep version one local-only and Windows 11-only.
- Resolve only %USERPROFILE%\.claude\projects and %USERPROFILE%\.codex\sessions, or those same relative paths beneath a synthetic test profile.
- Do not scan arbitrary user directories, WSL paths, or automatically discovered WSL distributions in this slice.
- Consider regular .json and .jsonl files only; never read their contents during discovery.
- Enforce both a maximum selected-file count and a maximum selected-byte total on every discovery call.
- Reject symlink/reparse-point roots, directories, and files, and reject any path that is not contained beneath the fixed provider root.
- Return only sanitized provider-relative layout metadata, file kind, and byte size. Do not expose absolute paths, source identifiers, timestamps, raw records, conversational content, credentials, repository contents, or working directories.
- Keep the actual validated filesystem path private to Rust and omit Debug, Serialize, and Clone implementations from the opaque file handle so it cannot cross a frontend or diagnostics boundary accidentally.
- Keep Claude and Codex outcomes independent: a missing, blocked, invalid, or limited root for one provider must not prevent a result for the other provider.
- Leave src-tauri/src/sources/file_watcher.rs as its existing scaffold until discovery and collection contracts are stable.
- Keep provider-specific parsing in src-tauri/src/providers/; this slice does not call readers or move parsing into sources/.
- Add no network client, telemetry, sidecar, background service, frontend state library, CSS framework, ORM, or new dependency.
- Follow the handoff's instruction to continue in the primary session; do not dispatch subagents for implementation.
- Execute implementation on feat/source-discovery created from the reviewed and updated dev branch; do not modify main or stage local .claude/ settings.
- Use test-first changes and run the narrowest Rust test after each behavior change before the broader repository gates.

## File Map

- src-tauri/src/sources/provider_roots.rs: fixed provider-relative native-root mapping, root validation, sanitized root identity, and private filesystem-root handle.
- src-tauri/src/sources/session_files.rs: bounded recursive enumeration, regular-file filtering, provider-relative layout sanitization, independent provider results, and opaque discovered-file metadata.
- src-tauri/src/utils/safe_paths.rs: reusable lexical containment, parent-traversal rejection, and Windows symlink/reparse-point checks with fixed error categories.
- src-tauri/tests/source_discovery.rs: synthetic-profile integration tests for roots, bounds, file selection, privacy, independence, WSL exclusion, and Windows reparse-point rejection.

Do not modify src-tauri/src/sources/mod.rs, src-tauri/src/sources/file_watcher.rs, provider readers, database modules, frontend files, or the Tauri command surface for this plan. The existing module indexes already expose the two source modules and safe_paths.

## Execution Prerequisite

The current handoff says the probe and provider-reader branches must be reviewed before this work is based on dev. Before starting Task 1, verify that those reviewed changes are available in dev, then create the feature branch:

~~~powershell
git status --short --branch
git branch --show-current
git log --oneline --decorate -8
git switch dev
git pull --ff-only origin dev
git switch -c feat/source-discovery
git status --short --branch
~~~

Expected: the new branch is based on the updated dev; existing .claude/ settings remain outside the staged implementation scope. If the prerequisite review/merge has not happened, stop before changing source files and complete that review through the normal branch workflow.

The profile_root parameter exists so integration tests can inject a synthetic profile. The eventual native call site supplies the current Windows USERPROFILE directory; no function in this slice accepts an arbitrary provider-root override or invokes WSL.

---

### Task 1: Specify fixed roots and safe-path behavior with failing tests

**Files:**
- Create: src-tauri/tests/source_discovery.rs

**Interfaces:**
- Consumes: existing token_tracing_widget_lib::types::provider::Provider and the source/utils module exports.
- Produces: the first executable contract for native_root_relative, resolve_native_root, and join_under_root; later tasks implement these exact names and return types.

- [ ] **Step 1: Write the failing synthetic-profile tests**

Create src-tauri/tests/source_discovery.rs with the root and safe-path tests below. The test intentionally uses a temporary profile rather than the real user profile, and it never writes provider records for these root-contract tests.

~~~rust
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use token_tracing_widget_lib::sources::provider_roots::{
    native_root_relative, resolve_native_root,
};
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::utils::safe_paths::{
    join_under_root, SafePathError,
};

fn synthetic_provider_root(profile: &Path, provider: Provider) -> PathBuf {
    profile.join(native_root_relative(provider))
}

#[test]
fn native_roots_are_fixed_beneath_the_synthetic_profile() {
    let profile = tempdir().expect("synthetic profile should be created");

    let claude_root = synthetic_provider_root(profile.path(), Provider::Claude);
    let codex_root = synthetic_provider_root(profile.path(), Provider::Codex);
    fs::create_dir_all(&claude_root).expect("Claude root should be created");
    fs::create_dir_all(&codex_root).expect("Codex root should be created");

    assert_eq!(native_root_relative(Provider::Claude), ".claude/projects");
    assert_eq!(native_root_relative(Provider::Codex), ".codex/sessions");

    let claude = resolve_native_root(profile.path(), Provider::Claude)
        .expect("Claude native root should resolve");
    let codex = resolve_native_root(profile.path(), Provider::Codex)
        .expect("Codex native root should resolve");

    assert_eq!(claude.provider(), Provider::Claude);
    assert_eq!(claude.relative_path(), ".claude/projects");
    assert_eq!(codex.provider(), Provider::Codex);
    assert_eq!(codex.relative_path(), ".codex/sessions");
}

#[test]
fn safe_join_rejects_parent_traversal() {
    let root = Path::new("synthetic-profile");

    assert_eq!(
        join_under_root(root, Path::new("..").join("outside")),
        Err(SafePathError::ParentTraversal)
    );
    assert_eq!(
        join_under_root(root, Path::new("nested/../../outside")),
        Err(SafePathError::ParentTraversal)
    );
}

#[cfg(windows)]
#[test]
fn safe_join_rejects_drive_and_unc_paths() {
    let root = Path::new(r"C:\synthetic-profile");

    assert_eq!(
        join_under_root(root, Path::new(r"D:\outside")),
        Err(SafePathError::AbsolutePath)
    );
    assert_eq!(
        join_under_root(root, Path::new(r"\\server\share\outside")),
        Err(SafePathError::AbsolutePath)
    );
}
~~~

- [ ] **Step 2: Run the focused test and verify the red state**

Run:

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery
~~~

Expected: FAIL during integration-test compilation because native_root_relative, resolve_native_root, join_under_root, and SafePathError do not exist yet. No source file should be read by the failing test.

---

### Task 2: Implement fixed native roots and safe path validation

**Files:**
- Modify: src-tauri/src/sources/provider_roots.rs
- Modify: src-tauri/src/utils/safe_paths.rs
- Test: src-tauri/tests/source_discovery.rs

**Interfaces:**
- Consumes: the failing root/path tests from Task 1 and types::provider::Provider.
- Produces: RootError, ProviderRoot, native_root_relative, resolve_native_root, SafePathError, join_under_root, and validate_existing_path for session_files.rs and later collection code.

- [ ] **Step 1: Define fixed root and path-error contracts**

Replace the scaffold bodies with these contracts. Keep all error variants payload-free so Display can return only a stable category and never include a path.

~~~rust
// src-tauri/src/sources/provider_roots.rs
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::provider::Provider;
use crate::utils::safe_paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootError {
    NotDetected,
    PermissionDenied,
    InvalidRoot,
    UnsafePath,
    Io,
}

impl fmt::Display for RootError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::NotDetected => "not_detected",
            Self::PermissionDenied => "permission_denied",
            Self::InvalidRoot => "invalid_root",
            Self::UnsafePath => "unsafe_path",
            Self::Io => "io",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for RootError {}

pub struct ProviderRoot {
    provider: Provider,
    relative_path: &'static str,
    filesystem_path: PathBuf,
}

impl ProviderRoot {
    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn relative_path(&self) -> &'static str {
        self.relative_path
    }

    pub(crate) fn filesystem_path(&self) -> &Path {
        &self.filesystem_path
    }
}

pub fn native_root_relative(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => ".claude/projects",
        Provider::Codex => ".codex/sessions",
    }
}

pub fn resolve_native_root(
    profile_root: &Path,
    provider: Provider,
) -> Result<ProviderRoot, RootError> {
    let profile_metadata = fs::symlink_metadata(profile_root).map_err(map_io_error)?;
    if !profile_metadata.is_dir() {
        return Err(RootError::InvalidRoot);
    }

    let relative_path = native_root_relative(provider);
    let filesystem_path = safe_paths::join_under_root(profile_root, Path::new(relative_path))
        .map_err(map_path_error)?;
    let metadata = fs::symlink_metadata(&filesystem_path).map_err(map_io_error)?;
    if !metadata.is_dir() {
        return Err(RootError::InvalidRoot);
    }
    safe_paths::validate_existing_path(profile_root, &filesystem_path)
        .map_err(map_path_error)?;

    Ok(ProviderRoot {
        provider,
        relative_path,
        filesystem_path,
    })
}

fn map_io_error(error: std::io::Error) -> RootError {
    match error.kind() {
        std::io::ErrorKind::NotFound => RootError::NotDetected,
        std::io::ErrorKind::PermissionDenied => RootError::PermissionDenied,
        _ => RootError::Io,
    }
}

fn map_path_error(error: safe_paths::SafePathError) -> RootError {
    match error {
        safe_paths::SafePathError::ReparsePoint
        | safe_paths::SafePathError::AbsolutePath
        | safe_paths::SafePathError::ParentTraversal
        | safe_paths::SafePathError::OutsideRoot => RootError::UnsafePath,
        safe_paths::SafePathError::NotDirectory => RootError::InvalidRoot,
        safe_paths::SafePathError::Io => RootError::Io,
    }
}
~~~

~~~rust
// src-tauri/src/utils/safe_paths.rs
use std::fmt;
use std::fs::{self, Metadata};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafePathError {
    AbsolutePath,
    ParentTraversal,
    OutsideRoot,
    ReparsePoint,
    NotDirectory,
    Io,
}

impl fmt::Display for SafePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::AbsolutePath => "absolute_path",
            Self::ParentTraversal => "parent_traversal",
            Self::OutsideRoot => "outside_root",
            Self::ReparsePoint => "reparse_point",
            Self::NotDirectory => "not_directory",
            Self::Io => "io",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for SafePathError {}

pub fn join_under_root(root: &Path, relative: &Path) -> Result<PathBuf, SafePathError> {
    for component in relative.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                return Err(SafePathError::AbsolutePath);
            }
            Component::ParentDir => return Err(SafePathError::ParentTraversal),
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    let candidate = root.join(relative);
    if !candidate.starts_with(root) {
        return Err(SafePathError::OutsideRoot);
    }
    Ok(candidate)
}

pub fn validate_existing_path(root: &Path, candidate: &Path) -> Result<(), SafePathError> {
    let relative = candidate
        .strip_prefix(root)
        .map_err(|_| SafePathError::OutsideRoot)?;
    let mut current = root.to_path_buf();
    validate_component(&current)?;

    let components: Vec<_> = relative.components().collect();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(SafePathError::ParentTraversal);
        };
        current.push(name);
        validate_component(&current)?;
        if index + 1 < components.len() && !fs::symlink_metadata(&current)
            .map_err(|_| SafePathError::Io)?
            .is_dir()
        {
            return Err(SafePathError::NotDirectory);
        }
    }
    Ok(())
}

fn validate_component(path: &Path) -> Result<(), SafePathError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => SafePathError::Io,
        _ => SafePathError::Io,
    })?;
    if is_reparse_point(&metadata) {
        return Err(SafePathError::ReparsePoint);
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse_point(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink()
}
~~~

The implementation must not call canonicalize: canonicalization follows links before they can be rejected. All checks use symlink_metadata, lexical component validation, and the Windows FILE_ATTRIBUTE_REPARSE_POINT bit.

- [ ] **Step 2: Run the root/path tests and verify they pass**

Run:

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery
~~~

Expected: PASS for the synthetic-root and parent/absolute-path tests. The absolute drive/UNC test is compiled only on Windows. No public method or formatted error contains the temporary profile's absolute path.

- [ ] **Step 3: Run formatting and commit the root boundary**

Run:

~~~powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
git diff --check
~~~

Expected: both commands exit 0.

Commit:

~~~powershell
git add src-tauri/src/sources/provider_roots.rs src-tauri/src/utils/safe_paths.rs src-tauri/tests/source_discovery.rs
git diff --cached --check
git commit -m "feat: define safe native provider roots"

~~~


---

### Task 3: Specify bounded file discovery and sanitized metadata with failing tests

**Files:**
- Modify: src-tauri/tests/source_discovery.rs

**Interfaces:**
- Consumes: Provider, native_root_relative, and the safe path boundary from Tasks 1-2.
- Produces: failing tests for DiscoveryLimits, DiscoveryStatus, SessionFileKind, DiscoveredSessionFile, DiscoveryResult, discover_provider, and discover_native_sources.

- [ ] **Step 1: Append the synthetic-profile discovery tests**

Append the following helpers and tests to src-tauri/tests/source_discovery.rs. They use only synthetic file names and content. The content is written solely to prove that discovery never needs to parse or return it.

~~~rust
use token_tracing_widget_lib::sources::session_files::{
    discover_native_sources, discover_provider, DiscoveryLimits, DiscoveryStatus,
    SessionFileKind,
};

fn create_file(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture parent should be created");
    }
    fs::write(path, contents).expect("fixture file should be written");
}

fn limits(max_files: usize, max_total_bytes: u64) -> DiscoveryLimits {
    DiscoveryLimits::new(max_files, max_total_bytes)
}

#[test]
fn discovery_returns_only_regular_json_and_jsonl_files() {
    let profile = tempdir().expect("synthetic profile should be created");
    let root = synthetic_provider_root(profile.path(), Provider::Claude);
    create_file(&root.join("workspace-alpha").join("one.jsonl"), b"one");
    create_file(&root.join("workspace-alpha").join("two.json"), b"two");
    create_file(&root.join("workspace-alpha").join("ignored.txt"), b"ignored");
    fs::create_dir_all(root.join("folder.json")).expect("directory fixture should be created");

    let result = discover_provider(profile.path(), Provider::Claude, limits(10, 1_000));

    assert_eq!(result.status(), DiscoveryStatus::Detected);
    assert_eq!(result.files().len(), 2);
    let mut kinds: Vec<_> = result.files().iter().map(|file| file.kind()).collect();
    kinds.sort_by_key(|kind| match kind {
        SessionFileKind::Json => 0,
        SessionFileKind::Jsonl => 1,
    });
    assert_eq!(kinds, vec![SessionFileKind::Json, SessionFileKind::Jsonl]);
}

#[test]
fn discovery_sanitizes_relative_layout_metadata() {
    let profile = tempdir().expect("synthetic profile should be created");
    let root = synthetic_provider_root(profile.path(), Provider::Claude);
    let source_file = root
        .join("private-workspace-name")
        .join("session-real-identifier.jsonl");
    let private_contents = br#"{"prompt":"private prompt","cwd":"C:\\private-repository"}"#;
    create_file(&source_file, private_contents);

    let result = discover_provider(profile.path(), Provider::Claude, limits(10, 1_000));
    let file = result.files().first().expect("one candidate should be returned");

    assert_eq!(file.kind(), SessionFileKind::Jsonl);
    assert_eq!(file.size_bytes(), private_contents.len() as u64);
    assert_eq!(file.relative_pattern(), "<segment>/<file>.jsonl");
    assert!(!file.relative_pattern().contains("private-workspace-name"));
    assert!(!file.relative_pattern().contains("real-identifier"));
    assert!(!file.relative_pattern().contains("private-repository"));
}

#[test]
fn discovery_enforces_file_and_byte_limits_before_selection() {
    let profile = tempdir().expect("synthetic profile should be created");
    let root = synthetic_provider_root(profile.path(), Provider::Codex);
    for index in 0..4 {
        create_file(
            &root.join(format!("day-{index}")).join("session.jsonl"),
            b"abc",
        );
    }

    let result = discover_provider(profile.path(), Provider::Codex, limits(2, 6));

    assert_eq!(result.files().len(), 2);
    assert_eq!(result.total_bytes(), 6);
    assert_eq!(result.status(), DiscoveryStatus::LimitReached);
}

#[test]
fn discovery_skips_a_file_that_would_exceed_the_byte_limit() {
    let profile = tempdir().expect("synthetic profile should be created");
    let root = synthetic_provider_root(profile.path(), Provider::Codex);
    create_file(&root.join("too-large.jsonl"), b"1234567");
    create_file(&root.join("small.jsonl"), b"12");

    let result = discover_provider(profile.path(), Provider::Codex, limits(10, 4));

    assert_eq!(result.files().len(), 1);
    assert_eq!(result.total_bytes(), 2);
    assert_eq!(result.status(), DiscoveryStatus::LimitReached);
}
~~~

- [ ] **Step 2: Run the discovery tests and verify the red state**

Run:

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery
~~~

Expected: FAIL during integration-test compilation because the session_files discovery contracts do not exist. Keep the tests as written; after Task 4 supplies the contracts, rerun the complete source_discovery test target and require every root, path, and discovery assertion to pass.

---

### Task 4: Implement bounded, metadata-only session-file discovery

**Files:**
- Modify: src-tauri/src/sources/session_files.rs
- Test: src-tauri/tests/source_discovery.rs

**Interfaces:**
- Consumes: ProviderRoot and RootError from provider_roots.rs, join_under_root and validate_existing_path from safe_paths.rs, and the failing synthetic tests from Task 3.
- Produces: public DiscoveryLimits, DiscoveryStatus, SessionFileKind, DiscoveredSessionFile, DiscoveryResult, discover_provider, and discover_native_sources; only DiscoveredSessionFile::filesystem_path is pub(crate) for future Rust collection code.

- [ ] **Step 1: Define the discovery result types without leaking paths**

Add these exact types and methods to session_files.rs. Do not derive Debug, Serialize, or Clone for DiscoveredSessionFile or DiscoveryResult; the private filesystem_path must not appear in accidental diagnostics or frontend payloads.

~~~rust
use std::fs;
use std::path::{Path, PathBuf};

use crate::sources::provider_roots::{resolve_native_root, ProviderRoot, RootError};
use crate::types::provider::Provider;
use crate::utils::safe_paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryLimits {
    pub max_files: usize,
    pub max_total_bytes: u64,
}

impl DiscoveryLimits {
    pub const fn new(max_files: usize, max_total_bytes: u64) -> Self {
        Self {
            max_files,
            max_total_bytes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryStatus {
    Detected,
    NotDetected,
    PermissionDenied,
    InvalidRoot,
    Unavailable,
    LimitReached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SessionFileKind {
    Json,
    Jsonl,
}

pub struct DiscoveredSessionFile {
    filesystem_path: PathBuf,
    relative_pattern: String,
    kind: SessionFileKind,
    size_bytes: u64,
}

impl DiscoveredSessionFile {
    pub fn relative_pattern(&self) -> &str {
        &self.relative_pattern
    }

    pub fn kind(&self) -> SessionFileKind {
        self.kind
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub(crate) fn filesystem_path(&self) -> &Path {
        &self.filesystem_path
    }
}

pub struct DiscoveryResult {
    provider: Provider,
    root_relative: &'static str,
    status: DiscoveryStatus,
    files: Vec<DiscoveredSessionFile>,
    total_bytes: u64,
    rejected_entries: u64,
}

impl DiscoveryResult {
    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn root_relative(&self) -> &'static str {
        self.root_relative
    }

    pub fn status(&self) -> DiscoveryStatus {
        self.status
    }

    pub fn files(&self) -> &[DiscoveredSessionFile] {
        &self.files
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub fn rejected_entries(&self) -> u64 {
        self.rejected_entries
    }
}
~~~

Do not add a Default implementation or an unlimited fallback for DiscoveryLimits. Every caller must pass explicit file and byte bounds; choosing the production collection cadence and thresholds belongs to the later collection-core plan, which is outside this handoff.

- [ ] **Step 2: Implement fixed-root result mapping and independent provider calls**

Add these functions. Each provider call resolves and walks only its own fixed root; discover_native_sources must not use ? across providers or return a single combined error.

~~~rust
pub fn discover_native_sources(
    profile_root: &Path,
    limits: DiscoveryLimits,
) -> [DiscoveryResult; 2] {
    [
        discover_provider(profile_root, Provider::Claude, limits),
        discover_provider(profile_root, Provider::Codex, limits),
    ]
}

pub fn discover_provider(
    profile_root: &Path,
    provider: Provider,
    limits: DiscoveryLimits,
) -> DiscoveryResult {
    let root_relative = crate::sources::provider_roots::native_root_relative(provider);
    let root = match resolve_native_root(profile_root, provider) {
        Ok(root) => root,
        Err(error) => return empty_result(provider, root_relative, status_for_root_error(error)),
    };

    walk_root(&root, limits)
}

fn empty_result(
    provider: Provider,
    root_relative: &'static str,
    status: DiscoveryStatus,
) -> DiscoveryResult {
    DiscoveryResult {
        provider,
        root_relative,
        status,
        files: Vec::new(),
        total_bytes: 0,
        rejected_entries: 0,
    }
}

fn status_for_root_error(error: RootError) -> DiscoveryStatus {
    match error {
        RootError::NotDetected => DiscoveryStatus::NotDetected,
        RootError::PermissionDenied => DiscoveryStatus::PermissionDenied,
        RootError::InvalidRoot | RootError::UnsafePath => DiscoveryStatus::InvalidRoot,
        RootError::Io => DiscoveryStatus::Unavailable,
    }
}
~~~

- [ ] **Step 3: Implement bounded recursive enumeration without reading content**

Implement walk_root with the following exact rules:

~~~rust
use std::time::{SystemTime, UNIX_EPOCH};

struct LocatedFile {
    file: DiscoveredSessionFile,
    modified_at: SystemTime,
}

fn walk_root(root: &ProviderRoot, limits: DiscoveryLimits) -> DiscoveryResult {
    let mut directories = vec![root.filesystem_path().to_path_buf()];
    let mut selected = Vec::<LocatedFile>::new();
    let mut total_bytes = 0_u64;
    let mut rejected_entries = 0_u64;
    let mut permission_seen = false;
    let mut io_seen = false;
    let mut limit_reached = limits.max_files == 0 || limits.max_total_bytes == 0;

    'walk: while let Some(directory) = directories.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                permission_seen |= error.kind() == std::io::ErrorKind::PermissionDenied;
                io_seen |= error.kind() != std::io::ErrorKind::PermissionDenied;
                continue;
            }
        };

        for entry in entries {
            if selected.len() >= limits.max_files {
                limit_reached = true;
                break 'walk;
            }

            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    permission_seen |= error.kind() == std::io::ErrorKind::PermissionDenied;
                    io_seen |= error.kind() != std::io::ErrorKind::PermissionDenied;
                    continue;
                }
            };
            let candidate_path = entry.path();

            if safe_paths::validate_existing_path(root.filesystem_path(), &candidate_path)
                .is_err()
            {
                rejected_entries = rejected_entries.saturating_add(1);
                continue;
            }

            let metadata = match fs::symlink_metadata(&candidate_path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    permission_seen |= error.kind() == std::io::ErrorKind::PermissionDenied;
                    io_seen |= error.kind() != std::io::ErrorKind::PermissionDenied;
                    continue;
                }
            };

            if metadata.is_dir() {
                directories.push(candidate_path);
                continue;
            }
            if !metadata.is_file() {
                continue;
            }

            let Some(kind) = session_file_kind(&candidate_path) else {
                continue;
            };
            let size_bytes = metadata.len();
            if total_bytes.saturating_add(size_bytes) > limits.max_total_bytes {
                limit_reached = true;
                continue;
            }

            let relative_pattern = sanitized_relative_pattern(
                root.filesystem_path(),
                &candidate_path,
                kind,
            );
            let Ok(relative_pattern) = relative_pattern else {
                rejected_entries = rejected_entries.saturating_add(1);
                continue;
            };

            selected.push(LocatedFile {
                file: DiscoveredSessionFile {
                    filesystem_path: candidate_path,
                    relative_pattern,
                    kind,
                    size_bytes,
                },
                modified_at: metadata.modified().unwrap_or(UNIX_EPOCH),
            });
            total_bytes = total_bytes.saturating_add(size_bytes);
        }
    }

    selected.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.file.relative_pattern.cmp(&right.file.relative_pattern))
    });
    let files = selected.into_iter().map(|located| located.file).collect();
    let status = if permission_seen {
        DiscoveryStatus::PermissionDenied
    } else if io_seen {
        DiscoveryStatus::Unavailable
    } else if limit_reached {
        DiscoveryStatus::LimitReached
    } else {
        DiscoveryStatus::Detected
    };

    DiscoveryResult {
        provider: root.provider(),
        root_relative: root.relative_path(),
        status,
        files,
        total_bytes,
        rejected_entries,
    }
}
~~~

session_file_kind must inspect only the extension and use ASCII case-insensitive matching:

~~~rust
fn session_file_kind(path: &Path) -> Option<SessionFileKind> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "json" => Some(SessionFileKind::Json),
        "jsonl" => Some(SessionFileKind::Jsonl),
        _ => None,
    }
}
~~~

Do not call fs::read, File::open, serde_json, or any provider reader in this module. metadata.modified() is used only for in-memory ordering and is never stored in DiscoveredSessionFile or DiscoveryResult.

- [ ] **Step 4: Implement provider-relative layout sanitization**

Implement sanitized_relative_pattern so every normal directory component becomes <segment> except an all-ASCII-digit component, which becomes <number>, and the final file component becomes <file>.json or <file>.jsonl. Join the markers with / regardless of Windows separator style.

~~~rust
use std::path::Component;

fn sanitized_relative_pattern(
    root: &Path,
    candidate: &Path,
    kind: SessionFileKind,
) -> Result<String, safe_paths::SafePathError> {
    let relative = candidate
        .strip_prefix(root)
        .map_err(|_| safe_paths::SafePathError::OutsideRoot)?;
    let components: Vec<_> = relative.components().collect();
    if components.is_empty() {
        return Err(safe_paths::SafePathError::OutsideRoot);
    }

    let extension = match kind {
        SessionFileKind::Json => "json",
        SessionFileKind::Jsonl => "jsonl",
    };
    let last = components.len() - 1;
    let mut markers = Vec::with_capacity(components.len());
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(safe_paths::SafePathError::ParentTraversal);
        };
        if index == last {
            markers.push(format!("<file>.{extension}"));
        } else if name.to_str().is_some_and(|value| {
            !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
        }) {
            markers.push("<number>".to_owned());
        } else {
            markers.push("<segment>".to_owned());
        }
    }
    Ok(markers.join("/"))
}
~~~

- [ ] **Step 5: Run the bounded discovery tests**

Run:

~~~powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery
~~~

Expected: PASS for fixed roots, parent/absolute rejection, regular JSON/JSONL selection, sanitized layout patterns, and file/byte bounds. The test process must not print any synthetic private content or absolute temporary path.

- [ ] **Step 6: Commit the bounded discovery implementation**

Run:

~~~powershell
git diff --check
git add src-tauri/src/sources/session_files.rs src-tauri/tests/source_discovery.rs
git diff --cached --check
git commit -m "feat: add bounded provider session discovery"

~~~


---

### Task 5: Add independent-provider, WSL-exclusion, and reparse-point regression proof

**Files:**
- Modify: src-tauri/tests/source_discovery.rs
- Modify: src-tauri/src/sources/session_files.rs only if a failing regression exposes a contract violation
- Modify: src-tauri/src/sources/provider_roots.rs or src-tauri/src/utils/safe_paths.rs only if the matching path/root regression requires it

**Interfaces:**
- Consumes: the discovery API from Task 4.
- Produces: proof that missing/invalid roots are independent, arbitrary siblings and WSL-shaped paths are ignored, and symlink/reparse entries cannot escape the fixed root.

- [ ] **Step 1: Write the provider-independence and WSL-exclusion tests**

Append these tests:

~~~rust
#[test]
fn provider_results_are_independent() {
    let profile = tempdir().expect("synthetic profile should be created");
    let codex_root = synthetic_provider_root(profile.path(), Provider::Codex);
    create_file(&codex_root.join("session.jsonl"), b"codex metadata only");

    let results = discover_native_sources(profile.path(), limits(10, 1_000));
    let claude = results
        .iter()
        .find(|result| result.provider() == Provider::Claude)
        .expect("Claude result should exist");
    let codex = results
        .iter()
        .find(|result| result.provider() == Provider::Codex)
        .expect("Codex result should exist");

    assert_eq!(claude.status(), DiscoveryStatus::NotDetected);
    assert_eq!(claude.files().len(), 0);
    assert_eq!(codex.status(), DiscoveryStatus::Detected);
    assert_eq!(codex.files().len(), 1);
}

#[test]
fn discovery_does_not_scan_arbitrary_siblings_or_wsl_shaped_paths() {
    let profile = tempdir().expect("synthetic profile should be created");
    create_file(
        &profile.path().join("unrelated").join("secret.jsonl"),
        b"unrelated",
    );
    create_file(
        &profile
            .path()
            .join("wsl.localhost")
            .join("distribution")
            .join("home")
            .join("user")
            .join("session.jsonl"),
        b"wsl-shaped",
    );
    let codex_root = synthetic_provider_root(profile.path(), Provider::Codex);
    create_file(&codex_root.join("native.jsonl"), b"native");

    let result = discover_provider(profile.path(), Provider::Codex, limits(10, 1_000));

    assert_eq!(result.files().len(), 1);
    assert_eq!(result.files()[0].relative_pattern(), "<file>.jsonl");
}
~~~

- [ ] **Step 2: Run the new tests and confirm the expected red state where necessary**

Run:

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery provider_results_are_independent
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery discovery_does_not_scan_arbitrary_siblings_or_wsl_shaped_paths
~~~

Expected: the tests pass if the Task 4 implementation already maps missing roots independently and resolves only fixed relative roots. If either test fails, keep the failure output limited to the fixed category/assertion and adjust only the responsible source boundary before continuing.

- [ ] **Step 3: Add a Windows reparse-point escape test**

Append this Windows-only test. It creates a synthetic outside directory, places a JSONL file there, and creates a directory symlink inside the fixed provider root. The discovery walk must inspect the link with symlink_metadata, reject it, and never enumerate the outside file.

~~~rust
#[cfg(windows)]
#[test]
fn discovery_rejects_reparse_point_escape() {
    use std::os::windows::fs::symlink_dir;

    let profile = tempdir().expect("synthetic profile should be created");
    let outside = tempdir().expect("outside fixture should be created");
    create_file(&outside.path().join("escaped.jsonl"), b"outside");

    let root = synthetic_provider_root(profile.path(), Provider::Claude);
    fs::create_dir_all(&root).expect("Claude root should be created");
    symlink_dir(outside.path(), root.join("linked-outside"))
        .expect("Windows test environment should create a directory symlink");

    let result = discover_provider(profile.path(), Provider::Claude, limits(10, 1_000));

    assert!(result.files().is_empty());
    assert!(result.rejected_entries() >= 1);
    assert_eq!(result.status(), DiscoveryStatus::Detected);
}
~~~

- [ ] **Step 4: Run the reparse/path regression tests**

Run:

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery safe_join_rejects
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery discovery_rejects_reparse_point_escape
~~~

Expected: path rejection tests pass. On Windows, the symlink test passes with an empty result and at least one rejected entry; no outside filename or content appears in the result. The product remains Windows-only, so the reparse test is not compiled on non-Windows hosts.

- [ ] **Step 5: Commit the regression proof**

Run:

~~~powershell
git diff --check
git add src-tauri/src/sources/provider_roots.rs src-tauri/src/sources/session_files.rs src-tauri/src/utils/safe_paths.rs src-tauri/tests/source_discovery.rs
git diff --cached --check
git commit -m "test: prove isolated safe source discovery"
~~~

If the source files are unchanged, stage only src-tauri/tests/source_discovery.rs; never stage .claude/ or unrelated worktree files.

---

### Task 6: Run the repository gates and publish the bounded-discovery commit

**Files:**
- Review: src-tauri/src/sources/provider_roots.rs
- Review: src-tauri/src/sources/session_files.rs
- Review: src-tauri/src/utils/safe_paths.rs
- Review: src-tauri/tests/source_discovery.rs

**Interfaces:**
- Consumes: the completed source-discovery implementation and its synthetic privacy tests.
- Produces: a verified feat/source-discovery branch with no frontend, watcher, database, provider-reader, or .claude/ changes.

- [ ] **Step 1: Run the focused Rust gate**

Run:

~~~powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
cargo check --manifest-path src-tauri/Cargo.toml
~~~

Expected: formatting, the source-discovery integration suite, all existing provider-reader/bootstrap/probe tests, and compilation all pass. The existing provider-reader tests must remain green and no provider reader file changes are required.

- [ ] **Step 2: Run the existing frontend and integrated-shell checks**

Run:

~~~powershell
npm test -- --run
npm run build
npm run tauri build -- --debug --no-bundle
~~~

Expected: frontend tests and production build pass, and the debug Tauri executable is produced. The source-discovery change must not add a command, capability, frontend payload, dependency, or background process.

- [ ] **Step 3: Review the privacy boundary and diff scope**

Run:

~~~powershell
git diff --check
git status --short --branch
git diff --stat dev...HEAD
git diff --name-only dev...HEAD
~~~

Expected: changed implementation paths are limited to src-tauri/src/sources/provider_roots.rs, src-tauri/src/sources/session_files.rs, src-tauri/src/utils/safe_paths.rs, and src-tauri/tests/source_discovery.rs (plus this plan if it is being carried on the branch). Confirm manually that:

- all public discovery values are provider-relative markers, file kind, byte size, fixed status, or counts;
- no public accessor, serializer, or formatter exposes or formats filesystem_path;
- no discovery code opens or parses a session file;
- no path is built from an arbitrary user-supplied root, WSL path, environment-wide scan, or directory outside the fixed provider root;
- .claude/ settings, src-tauri/target/, and unrelated work are absent from the staged diff.

- [ ] **Step 4: Commit only the verified source-discovery files**

Run:

~~~powershell
git add src-tauri/src/sources/provider_roots.rs src-tauri/src/sources/session_files.rs src-tauri/src/utils/safe_paths.rs src-tauri/tests/source_discovery.rs
git diff --cached --check
git diff --cached --name-only
git commit -m "feat: bound native provider source discovery"
~~~

Expected: the commit contains only the four implementation/test files. Do not push or merge main; publish feat/source-discovery through the normal review workflow after the user reviews the branch.

## Plan Self-Review

### Spec coverage

- Fixed native Claude and Codex roots are covered by Tasks 1-2 and the native_roots_are_fixed_beneath_the_synthetic_profile test.
- Provider-relative discovery metadata, regular JSON/JSONL filtering, bounded file/byte selection, and no content reads are covered by Tasks 3-4.
- Symlink/reparse-point rejection and lexical path containment are covered by Tasks 1-2 and Task 5.
- Independent provider health/results are covered by Task 5; missing Claude does not suppress a valid Codex result.
- WSL and arbitrary-directory exclusion are explicitly tested in Task 5 and are not implemented as automatic discovery paths.
- Metadata-only privacy is enforced by private non-serializable file handles, marker-only layout patterns, payload-free errors, and the private-content regression in Task 3.
- Rust ownership and existing Tauri/frontend boundaries remain unchanged because no command, watcher, database, provider-reader, or frontend file is in the file map.
- Restart checkpoints, reader invocation, delta conversion, deduplication, SQLite persistence, explicit WSL settings roots, watcher reconciliation, overlay state, and Windows smoke behavior are outside this handoff's source-discovery slice and are not silently folded into the plan.

### Type consistency

The plan uses one stable chain of names: ProviderRoot is returned by resolve_native_root; discover_provider consumes it internally and returns DiscoveryResult; discover_native_sources returns [DiscoveryResult; 2]; each result exposes &[DiscoveredSessionFile]; each file exposes relative_pattern(), kind(), and size_bytes(), while the private filesystem_path field is reachable only through the pub(crate) filesystem_path() method.
