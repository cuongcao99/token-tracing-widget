use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::tempdir;
use token_tracing_widget_lib::sources::provider_roots::{
    configured_root_path, native_root_relative, resolve_native_root,
};
use token_tracing_widget_lib::sources::session_files::{
    discover_configured_source, discover_configured_sources, discover_native_sources,
    discover_provider, DiscoveryLimits, DiscoveryStatus, SessionFileKind,
};
use token_tracing_widget_lib::sources::source_config::SourceConfig;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::utils::safe_paths::{join_under_root, SafePathError};

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
    assert_eq!(claude.configured_root_label(), ".claude/projects");
    assert_eq!(codex.provider(), Provider::Codex);
    assert_eq!(codex.configured_root_label(), ".codex/sessions");
}

#[test]
fn configured_root_path_can_open_a_missing_native_folder() {
    let profile = tempdir().expect("synthetic profile should be created");
    let config = SourceConfig::defaults(Provider::Codex);

    let path = configured_root_path(profile.path(), &config)
        .expect("the configured native path should be addressable");

    assert_eq!(path, profile.path().join(".codex/sessions"));
}

#[test]
fn safe_join_rejects_parent_traversal() {
    let root = Path::new("synthetic-profile");

    assert_eq!(
        join_under_root(root, &Path::new("..").join("outside")),
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
        join_under_root(root, Path::new(r"\server\share\outside")),
        Err(SafePathError::AbsolutePath)
    );
}

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
    create_file(
        &root.join("workspace-alpha").join("ignored.txt"),
        b"ignored",
    );
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
    let file = result
        .files()
        .first()
        .expect("one candidate should be returned");

    assert_eq!(file.kind(), SessionFileKind::Jsonl);
    assert_eq!(file.size_bytes(), private_contents.len() as u64);
    assert_eq!(file.relative_pattern(), "<segment>/<file>.jsonl");
    assert!(!file.relative_pattern().contains("private-workspace-name"));
    assert!(!file.relative_pattern().contains("real-identifier"));
    assert!(!file.relative_pattern().contains("private-repository"));
}

#[test]
fn discovery_enforces_file_and_byte_limits() {
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
fn unbounded_discovery_accepts_all_small_files() {
    let profile = tempdir().expect("synthetic profile should be created");
    let root = synthetic_provider_root(profile.path(), Provider::Codex);
    for index in 0..6 {
        create_file(
            &root.join(format!("session-{index}.jsonl")),
            b"metadata only",
        );
    }

    let result = discover_provider(
        profile.path(),
        Provider::Codex,
        limits(usize::MAX, 50 * 1024 * 1024),
    );

    assert_eq!(result.files().len(), 6);
    assert_eq!(result.status(), DiscoveryStatus::Detected);
}

#[test]
fn discovery_applies_byte_limit_after_newest_sorting() {
    let profile = tempdir().expect("synthetic profile should be created");
    let root = synthetic_provider_root(profile.path(), Provider::Codex);
    create_file(&root.join("a-old.jsonl"), b"o");
    std::thread::sleep(Duration::from_millis(50));
    create_file(&root.join("z-new.jsonl"), b"nn");

    let result = discover_provider(profile.path(), Provider::Codex, limits(usize::MAX, 2));

    assert_eq!(result.files().len(), 1);
    assert_eq!(result.files()[0].size_bytes(), 2);
    assert_eq!(result.total_bytes(), 2);
    assert_eq!(result.status(), DiscoveryStatus::LimitReached);
}

#[test]
fn newest_oversized_file_remains_discoverable_for_incremental_reading() {
    let profile = tempdir().expect("synthetic profile should be created");
    let root = synthetic_provider_root(profile.path(), Provider::Codex);
    create_file(&root.join("a-old.jsonl"), b"old");
    std::thread::sleep(Duration::from_millis(50));
    create_file(&root.join("z-new.jsonl"), b"newest");

    let result = discover_provider(profile.path(), Provider::Codex, limits(10, 4));

    assert_eq!(result.files().len(), 1);
    assert_eq!(result.files()[0].size_bytes(), 6);
    assert_eq!(result.total_bytes(), 6);
    assert_eq!(result.status(), DiscoveryStatus::LimitReached);
}

#[test]
fn discovery_skips_a_file_that_would_exceed_the_byte_limit() {
    let profile = tempdir().expect("synthetic profile should be created");
    let root = synthetic_provider_root(profile.path(), Provider::Codex);
    create_file(&root.join("too-large.jsonl"), b"1234567");
    std::thread::sleep(Duration::from_millis(50));
    create_file(&root.join("small.jsonl"), b"12");

    let result = discover_provider(profile.path(), Provider::Codex, limits(10, 4));

    assert_eq!(result.files().len(), 1);
    assert_eq!(result.total_bytes(), 2);
    assert_eq!(result.status(), DiscoveryStatus::LimitReached);
}

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
    assert!(claude.files().is_empty());
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

#[cfg(windows)]
#[test]
fn discovery_rejects_reparse_point_escape() {
    use std::os::windows::fs::symlink_dir;

    let profile = tempdir().expect("synthetic profile should be created");
    let outside = tempdir().expect("outside fixture should be created");
    create_file(&outside.path().join("escaped.jsonl"), b"outside");

    let root = synthetic_provider_root(profile.path(), Provider::Claude);
    fs::create_dir_all(&root).expect("Claude root should be created");
    let link = root.join("linked-outside");
    if symlink_dir(outside.path(), &link).is_err() {
        return;
    }

    let result = discover_provider(profile.path(), Provider::Claude, limits(10, 1_000));

    assert!(result.files().is_empty());
    assert!(result.rejected_entries() >= 1);
    assert_eq!(result.status(), DiscoveryStatus::Detected);
}

#[test]
fn explicit_existing_root_is_discovered_with_its_configured_label() {
    let profile = tempfile::tempdir().unwrap();
    let root = profile.path().join("custom-source");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("session.jsonl"), b"metadata only").unwrap();
    let config = SourceConfig::try_new(Provider::Claude, true, Some(root.clone())).unwrap();
    let label = root.to_string_lossy().into_owned();

    let result =
        discover_configured_source(profile.path(), &config, DiscoveryLimits::new(10, 1_000));

    assert_eq!(result.status(), DiscoveryStatus::Detected);
    assert_eq!(result.configured_root(), label);
    assert_eq!(result.files().len(), 1);
}

#[cfg(windows)]
#[test]
fn configured_discovery_keeps_windows_and_wsl_results_independent() {
    let profile = tempfile::tempdir().unwrap();
    let windows_root = profile.path().join("windows-source");
    fs::create_dir_all(&windows_root).unwrap();
    fs::write(windows_root.join("windows.jsonl"), b"metadata only").unwrap();
    let wsl_root = PathBuf::from(r"\\wsl.localhost\Ubuntu\home\tester\.claude\projects");
    let config = SourceConfig::try_new_with_roots(
        Provider::Claude,
        true,
        Some(windows_root.clone()),
        Some(wsl_root.clone()),
    )
    .unwrap();

    let results = discover_configured_sources(profile.path(), &config, limits(10, 1_000));

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].configured_root(), windows_root.to_string_lossy());
    assert_eq!(results[0].status(), DiscoveryStatus::Detected);
    assert_eq!(results[0].files().len(), 1);
    assert_eq!(results[1].configured_root(), wsl_root.to_string_lossy());
    assert!(matches!(
        results[1].status(),
        DiscoveryStatus::NotDetected | DiscoveryStatus::PermissionDenied
    ));
}

#[test]
fn missing_explicit_root_reports_not_detected_without_scanning_profile() {
    let profile = tempfile::tempdir().unwrap();
    let root = profile.path().join("not-mounted-yet");
    let config = SourceConfig::try_new(Provider::Claude, true, Some(root.clone())).unwrap();
    let label = root.to_string_lossy().into_owned();

    let result =
        discover_configured_source(profile.path(), &config, DiscoveryLimits::new(10, 1_000));

    assert_eq!(result.status(), DiscoveryStatus::NotDetected);
    assert!(result.files().is_empty());
    assert_eq!(result.configured_root(), label);
}

#[test]
fn disabled_source_never_walks_its_root() {
    let profile = tempfile::tempdir().unwrap();
    let root = profile.path().join("disabled-source");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("session.jsonl"), b"metadata only").unwrap();
    let config = SourceConfig::try_new(Provider::Claude, false, Some(root)).unwrap();

    let result =
        discover_configured_source(profile.path(), &config, DiscoveryLimits::new(10, 1_000));

    assert_eq!(result.status(), DiscoveryStatus::Disabled);
    assert!(result.files().is_empty());
}
