use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use token_tracing_widget_lib::sources::provider_roots::{
    native_root_relative, resolve_native_root,
};
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
    assert_eq!(claude.relative_path(), ".claude/projects");
    assert_eq!(codex.provider(), Provider::Codex);
    assert_eq!(codex.relative_path(), ".codex/sessions");
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
