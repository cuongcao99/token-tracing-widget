//! Known native and explicitly configured provider roots.

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
    #[allow(dead_code)]
    filesystem_path: PathBuf,
}

impl ProviderRoot {
    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn relative_path(&self) -> &'static str {
        self.relative_path
    }

    #[allow(dead_code)]
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
    safe_paths::validate_existing_path(profile_root, &filesystem_path).map_err(map_path_error)?;

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
