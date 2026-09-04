//! Known native and explicitly configured provider roots.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::sources::source_config::{SourceConfig, SourcePlatform};
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
    configured_root: String,
    filesystem_path: PathBuf,
}

impl ProviderRoot {
    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn configured_root(&self) -> &str {
        &self.configured_root
    }

    pub fn configured_root_label(&self) -> &str {
        self.configured_root()
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
    resolve_automatic_root(profile_root, provider)
}

pub fn resolve_configured_root(
    profile_root: &Path,
    config: &SourceConfig,
) -> Result<ProviderRoot, RootError> {
    let platform =
        if config.windows_root_override().is_some() || config.wsl_root_override().is_none() {
            SourcePlatform::Windows
        } else {
            SourcePlatform::Wsl
        };
    resolve_configured_root_for(profile_root, config, platform)
}

pub fn resolve_configured_roots(
    profile_root: &Path,
    config: &SourceConfig,
) -> Vec<(String, Result<ProviderRoot, RootError>)> {
    let provider = config.provider();
    let windows = match config.windows_root_override() {
        Some(path) => (
            path.to_string_lossy().into_owned(),
            resolve_explicit_root(provider, path, path.to_string_lossy().into_owned()),
        ),
        None => (
            native_root_relative(provider).to_owned(),
            resolve_automatic_root(profile_root, provider),
        ),
    };
    let mut roots = vec![windows];
    if let Some(path) = config.wsl_root_override() {
        let label = path.to_string_lossy().into_owned();
        roots.push((label.clone(), resolve_explicit_root(provider, path, label)));
    }
    roots
}

pub fn configured_root_path(
    profile_root: &Path,
    config: &SourceConfig,
) -> Result<PathBuf, RootError> {
    let platform =
        if config.windows_root_override().is_some() || config.wsl_root_override().is_none() {
            SourcePlatform::Windows
        } else {
            SourcePlatform::Wsl
        };
    configured_root_path_for(profile_root, config, platform)
}

pub fn configured_root_path_for(
    profile_root: &Path,
    config: &SourceConfig,
    platform: SourcePlatform,
) -> Result<PathBuf, RootError> {
    match platform {
        SourcePlatform::Windows => config
            .windows_root_override()
            .map(Path::to_path_buf)
            .map(Ok)
            .unwrap_or_else(|| {
                safe_paths::join_under_root(
                    profile_root,
                    Path::new(native_root_relative(config.provider())),
                )
                .map_err(map_path_error)
            }),
        SourcePlatform::Wsl => config
            .wsl_root_override()
            .map(Path::to_path_buf)
            .ok_or(RootError::NotDetected),
    }
}

pub fn watch_root_path(profile_root: &Path, config: &SourceConfig) -> Result<PathBuf, RootError> {
    let platform =
        if config.windows_root_override().is_some() || config.wsl_root_override().is_none() {
            SourcePlatform::Windows
        } else {
            SourcePlatform::Wsl
        };
    watch_root_path_for(profile_root, config, platform)
}

pub fn watch_root_paths(profile_root: &Path, config: &SourceConfig) -> Vec<PathBuf> {
    if !config.enabled() {
        return Vec::new();
    }

    let mut paths = Vec::with_capacity(2);
    if let Ok(path) = watch_root_path_for(profile_root, config, SourcePlatform::Windows) {
        paths.push(path);
    }
    if config.wsl_root_override().is_some() {
        if let Ok(path) = watch_root_path_for(profile_root, config, SourcePlatform::Wsl) {
            paths.push(path);
        }
    }
    paths
}

fn resolve_configured_root_for(
    profile_root: &Path,
    config: &SourceConfig,
    platform: SourcePlatform,
) -> Result<ProviderRoot, RootError> {
    match platform {
        SourcePlatform::Windows => match config.windows_root_override() {
            Some(path) => {
                let label = path.to_string_lossy().into_owned();
                resolve_explicit_root(config.provider(), path, label)
            }
            None => resolve_automatic_root(profile_root, config.provider()),
        },
        SourcePlatform::Wsl => match config.wsl_root_override() {
            Some(path) => {
                let label = path.to_string_lossy().into_owned();
                resolve_explicit_root(config.provider(), path, label)
            }
            None => Err(RootError::NotDetected),
        },
    }
}

fn watch_root_path_for(
    profile_root: &Path,
    config: &SourceConfig,
    platform: SourcePlatform,
) -> Result<PathBuf, RootError> {
    match platform {
        SourcePlatform::Windows if config.windows_root_override().is_some() => {
            resolve_configured_root_for(profile_root, config, platform)
                .map(|root| root.filesystem_path().to_path_buf())
        }
        SourcePlatform::Windows => resolve_existing_relative_root(
            profile_root,
            native_provider_root_relative(config.provider()),
        ),
        SourcePlatform::Wsl => resolve_configured_root_for(profile_root, config, platform)
            .map(|root| root.filesystem_path().to_path_buf()),
    }
}

pub fn native_provider_root_relative(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => ".claude",
        Provider::Codex => ".codex",
    }
}

fn resolve_automatic_root(
    profile_root: &Path,
    provider: Provider,
) -> Result<ProviderRoot, RootError> {
    let relative_path = native_root_relative(provider);
    let filesystem_path = resolve_existing_relative_root(profile_root, relative_path)?;

    Ok(ProviderRoot {
        provider,
        configured_root: relative_path.to_owned(),
        filesystem_path,
    })
}

fn resolve_existing_relative_root(
    profile_root: &Path,
    relative_path: &str,
) -> Result<PathBuf, RootError> {
    let profile_metadata = fs::symlink_metadata(profile_root).map_err(map_io_error)?;
    if !profile_metadata.is_dir() {
        return Err(RootError::InvalidRoot);
    }

    let filesystem_path = safe_paths::join_under_root(profile_root, Path::new(relative_path))
        .map_err(map_path_error)?;
    let metadata = fs::symlink_metadata(&filesystem_path).map_err(map_io_error)?;
    if !metadata.is_dir() {
        return Err(RootError::InvalidRoot);
    }
    safe_paths::validate_existing_path(profile_root, &filesystem_path).map_err(map_path_error)?;
    Ok(filesystem_path)
}

fn resolve_explicit_root(
    provider: Provider,
    path: &Path,
    configured_root: String,
) -> Result<ProviderRoot, RootError> {
    let metadata = fs::symlink_metadata(path).map_err(map_io_error)?;
    if !metadata.is_dir() {
        return Err(RootError::InvalidRoot);
    }
    safe_paths::validate_existing_path(path, path).map_err(map_path_error)?;

    Ok(ProviderRoot {
        provider,
        configured_root,
        filesystem_path: path.to_path_buf(),
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
