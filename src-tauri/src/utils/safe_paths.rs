//! Safe path normalization and boundary checks.

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
        let metadata = validate_component(&current)?;
        if index + 1 < components.len() && !metadata.is_dir() {
            return Err(SafePathError::NotDirectory);
        }
    }
    Ok(())
}

fn validate_component(path: &Path) -> Result<Metadata, SafePathError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| SafePathError::Io)?;
    if is_reparse_point(&metadata) {
        return Err(SafePathError::ReparsePoint);
    }
    Ok(metadata)
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
