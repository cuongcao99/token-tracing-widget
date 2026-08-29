//! Bounded discovery of provider session files.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

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
    modified_at_unix_ms: u64,
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

    pub(crate) fn modified_at_unix_ms(&self) -> u64 {
        self.modified_at_unix_ms
    }

    pub(crate) fn opaque_identity(&self, provider: Provider) -> String {
        let mut hasher = Sha256::new();
        hasher.update(provider.as_str().as_bytes());
        hasher.update([0]);
        hasher.update(self.filesystem_path.to_string_lossy().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    #[allow(dead_code)]
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
                record_io_error(&error, &mut permission_seen, &mut io_seen);
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
                    record_io_error(&error, &mut permission_seen, &mut io_seen);
                    continue;
                }
            };
            let candidate_path = entry.path();

            if safe_paths::validate_existing_path(root.filesystem_path(), &candidate_path).is_err()
            {
                rejected_entries = rejected_entries.saturating_add(1);
                continue;
            }

            let metadata = match fs::symlink_metadata(&candidate_path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    record_io_error(&error, &mut permission_seen, &mut io_seen);
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

            let relative_pattern =
                match sanitized_relative_pattern(root.filesystem_path(), &candidate_path, kind) {
                    Ok(pattern) => pattern,
                    Err(_) => {
                        rejected_entries = rejected_entries.saturating_add(1);
                        continue;
                    }
                };

            selected.push(LocatedFile {
                file: DiscoveredSessionFile {
                    filesystem_path: candidate_path,
                    relative_pattern,
                    kind,
                    size_bytes,
                    modified_at_unix_ms: system_time_to_unix_ms(
                        metadata.modified().unwrap_or(UNIX_EPOCH),
                    ),
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

fn system_time_to_unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn record_io_error(error: &std::io::Error, permission_seen: &mut bool, io_seen: &mut bool) {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        *permission_seen = true;
    } else {
        *io_seen = true;
    }
}

fn session_file_kind(path: &Path) -> Option<SessionFileKind> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "json" => Some(SessionFileKind::Json),
        "jsonl" => Some(SessionFileKind::Jsonl),
        _ => None,
    }
}

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
