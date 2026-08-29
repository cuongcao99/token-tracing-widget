#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{discover_candidates, provider_root, ProbeLimits};
    use crate::report::Provider;

    #[test]
    fn roots_are_fixed_beneath_the_supplied_profile() {
        let profile = std::path::Path::new(r"C:\synthetic-profile");
        assert_eq!(
            provider_root(profile, Provider::Claude),
            profile.join(".claude").join("projects")
        );
        assert_eq!(
            provider_root(profile, Provider::Codex),
            profile.join(".codex").join("sessions")
        );
    }

    #[test]
    fn discovery_obeys_file_and_byte_limits() {
        let profile = tempdir().unwrap();
        let root = provider_root(profile.path(), Provider::Codex);
        fs::create_dir_all(&root).unwrap();
        for index in 0..7 {
            fs::write(root.join(format!("session-{index}.jsonl")), b"{}\n").unwrap();
        }
        let result = discover_candidates(
            profile.path(),
            Provider::Codex,
            ProbeLimits {
                max_files: 5,
                max_bytes: 20,
                max_records: 50_000,
                max_record_bytes: 1_048_576,
            },
        );
        assert_eq!(result.candidates.len(), 5);
        assert_eq!(result.selected_bytes, 15);
        assert!(result.selected_bytes <= 20);
        assert!(result
            .candidates
            .iter()
            .all(|candidate| !candidate.layout_pattern.contains("session-")));
    }

    #[test]
    fn discovery_rejects_a_candidate_that_would_cross_the_byte_cap() {
        let profile = tempdir().unwrap();
        let root = provider_root(profile.path(), Provider::Codex);
        fs::create_dir_all(&root).unwrap();
        for index in 0..3 {
            fs::write(root.join(format!("candidate-{index}.jsonl")), b"{}\n").unwrap();
        }

        let result = discover_candidates(
            profile.path(),
            Provider::Codex,
            ProbeLimits {
                max_files: 5,
                max_bytes: 7,
                max_records: 50_000,
                max_record_bytes: 1_048_576,
            },
        );

        assert_eq!(result.candidates.len(), 2);
        assert_eq!(result.selected_bytes, 6);
    }

    #[test]
    fn containment_rejects_parent_escape_and_accepts_nested_paths() {
        let root = std::path::Path::new(r"C:\synthetic-profile\.codex\sessions");
        assert!(super::is_within_root(
            root,
            &root.join("nested").join("record.jsonl")
        ));
        assert!(!super::is_within_root(
            root,
            &root.join("..").join("outside.jsonl")
        ));
        assert!(!super::is_within_root(
            root,
            std::path::Path::new(r"C:\synthetic-profile\.codex\sessions-elsewhere\record.jsonl")
        ));
    }

    #[test]
    fn enumeration_failures_are_counted_without_exposing_paths() {
        let profile = tempdir().unwrap();
        let missing = profile.path().join("missing");
        let mut files = Vec::new();
        let mut errors = 0;

        super::collect_files(profile.path(), &missing, &mut files, 5, &mut errors);

        assert!(files.is_empty());
        assert_eq!(errors, 1);
    }

    #[test]
    fn discovery_inventory_is_bounded_before_candidate_selection() {
        let root = tempdir().unwrap();
        for index in 0..100 {
            fs::write(root.path().join(format!("session-{index}.jsonl")), b"{}\n").unwrap();
        }
        let mut files = Vec::new();
        let mut errors = 0;

        super::collect_files(root.path(), root.path(), &mut files, 5, &mut errors);

        assert_eq!(files.len(), 5);
        assert_eq!(errors, 0);
    }

    #[cfg(windows)]
    #[test]
    fn discovery_skips_symlinked_files_and_directories() {
        use std::os::windows::fs::{symlink_dir, symlink_file};

        let profile = tempdir().unwrap();
        let root = provider_root(profile.path(), Provider::Codex);
        fs::create_dir_all(&root).unwrap();
        let outside = tempdir().unwrap();
        let outside_file = outside.path().join("outside.jsonl");
        fs::write(&outside_file, b"{}\n").unwrap();
        fs::write(root.join("inside.jsonl"), b"{}\n").unwrap();

        let linked_file = root.join("linked.jsonl");
        if symlink_file(&outside_file, &linked_file).is_err() {
            return;
        }
        let linked_dir = root.join("linked-dir");
        if symlink_dir(outside.path(), &linked_dir).is_err() {
            return;
        }

        let result = discover_candidates(profile.path(), Provider::Codex, ProbeLimits::default());

        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.discovery_errors, 0);
    }
}
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::report::Provider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootState {
    Readable,
    NotDetected,
    PermissionDenied,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct ProbeLimits {
    pub max_files: usize,
    pub max_bytes: u64,
    pub max_records: u64,
    pub max_record_bytes: usize,
}

impl Default for ProbeLimits {
    fn default() -> Self {
        Self {
            max_files: 5,
            max_bytes: 50 * 1024 * 1024,
            max_records: 50_000,
            max_record_bytes: 1024 * 1024,
        }
    }
}

pub struct CandidateFile {
    pub(crate) path: PathBuf,
    pub layout_pattern: String,
    #[allow(dead_code)]
    pub size: u64,
}

pub struct DiscoveryResult {
    pub provider: Provider,
    pub root_state: RootState,
    pub candidates: Vec<CandidateFile>,
    pub selected_bytes: u64,
    pub discovery_errors: u64,
}

pub fn provider_root(profile_root: &Path, provider: Provider) -> PathBuf {
    match provider {
        Provider::Claude => profile_root.join(".claude").join("projects"),
        Provider::Codex => profile_root.join(".codex").join("sessions"),
    }
}

pub fn discover_candidates(
    profile_root: &Path,
    provider: Provider,
    limits: ProbeLimits,
) -> DiscoveryResult {
    let root = provider_root(profile_root, provider);
    let metadata = match fs::symlink_metadata(&root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return empty_result(provider, RootState::Error);
        }
        Ok(metadata) if metadata.is_dir() => metadata,
        Ok(_) => return empty_result(provider, RootState::Error),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return empty_result(provider, RootState::NotDetected);
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            return empty_result(provider, RootState::PermissionDenied);
        }
        Err(_) => return result_with_error(provider, RootState::Error),
    };
    let _ = metadata;
    match is_reparse_point(&root) {
        Ok(true) => return empty_result(provider, RootState::Error),
        Ok(false) => {}
        Err(_) => return result_with_error(provider, RootState::Error),
    }
    if let Err(error) = fs::read_dir(&root) {
        let state = match error.kind() {
            std::io::ErrorKind::NotFound => RootState::NotDetected,
            std::io::ErrorKind::PermissionDenied => RootState::PermissionDenied,
            _ => RootState::Error,
        };
        return if state == RootState::Error {
            result_with_error(provider, state)
        } else {
            empty_result(provider, state)
        };
    }

    let mut discovered = Vec::new();
    let mut discovery_error_count = 0;
    collect_files(
        &root,
        &root,
        &mut discovered,
        limits.max_files,
        &mut discovery_error_count,
    );
    discovered.sort_by(compare_discovered);

    let mut candidates = Vec::new();
    let mut selected_bytes = 0_u64;
    for discovered in discovered {
        if candidates.len() >= limits.max_files {
            break;
        }
        let Some(next_selected_bytes) = selected_bytes.checked_add(discovered.size) else {
            break;
        };
        if next_selected_bytes > limits.max_bytes {
            break;
        }
        selected_bytes = next_selected_bytes;
        candidates.push(CandidateFile {
            path: discovered.path,
            layout_pattern: discovered.layout_pattern,
            size: discovered.size,
        });
    }

    DiscoveryResult {
        provider,
        root_state: RootState::Readable,
        candidates,
        selected_bytes,
        discovery_errors: discovery_error_count,
    }
}

fn empty_result(provider: Provider, root_state: RootState) -> DiscoveryResult {
    DiscoveryResult {
        provider,
        root_state,
        candidates: Vec::new(),
        selected_bytes: 0,
        discovery_errors: 0,
    }
}

fn result_with_error(provider: Provider, root_state: RootState) -> DiscoveryResult {
    DiscoveryResult {
        provider,
        root_state,
        candidates: Vec::new(),
        selected_bytes: 0,
        discovery_errors: 1,
    }
}

struct DiscoveredFile {
    path: PathBuf,
    layout_pattern: String,
    size: u64,
    modified: SystemTime,
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<DiscoveredFile>,
    max_files: usize,
    discovery_error_count: &mut u64,
) {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => {
            increment_discovery_errors(discovery_error_count);
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                increment_discovery_errors(discovery_error_count);
                continue;
            }
        };
        let path = entry.path();
        if !is_within_root(root, &path) {
            increment_discovery_errors(discovery_error_count);
            continue;
        }
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => {
                increment_discovery_errors(discovery_error_count);
                continue;
            }
        };
        let is_reparse = match is_reparse_point(&path) {
            Ok(is_reparse) => is_reparse,
            Err(_) => {
                increment_discovery_errors(discovery_error_count);
                continue;
            }
        };
        if file_type.is_symlink() || is_reparse {
            continue;
        }
        if file_type.is_dir() {
            collect_files(root, &path, files, max_files, discovery_error_count);
            continue;
        }
        if !file_type.is_file() || !is_supported_extension(&path) {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                increment_discovery_errors(discovery_error_count);
                continue;
            }
        };
        let Ok(relative) = path.strip_prefix(root) else {
            increment_discovery_errors(discovery_error_count);
            continue;
        };
        let layout_pattern = layout_pattern(relative);
        retain_recent(
            files,
            DiscoveredFile {
                path,
                layout_pattern,
                size: metadata.len(),
                modified: metadata.modified().unwrap_or(UNIX_EPOCH),
            },
            max_files,
        );
    }
}

fn retain_recent(files: &mut Vec<DiscoveredFile>, file: DiscoveredFile, max_files: usize) {
    if max_files == 0 {
        return;
    }
    files.push(file);
    files.sort_by(compare_discovered);
    files.truncate(max_files);
}

fn compare_discovered(left: &DiscoveredFile, right: &DiscoveredFile) -> std::cmp::Ordering {
    right
        .modified
        .cmp(&left.modified)
        .then_with(|| left.layout_pattern.cmp(&right.layout_pattern))
        .then_with(|| left.path.cmp(&right.path))
}

fn increment_discovery_errors(count: &mut u64) {
    *count = count.saturating_add(1).min(1024);
}

fn is_within_root(root: &Path, candidate: &Path) -> bool {
    let Ok(relative) = candidate.strip_prefix(root) else {
        return false;
    };
    !relative.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    })
}

#[cfg(windows)]
fn is_reparse_point(path: &Path) -> std::io::Result<bool> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

#[cfg(not(windows))]
fn is_reparse_point(_path: &Path) -> std::io::Result<bool> {
    Ok(false)
}

fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("json") || extension.eq_ignore_ascii_case("jsonl")
        })
}

fn layout_pattern(relative: &Path) -> String {
    let components: Vec<_> = relative.iter().collect();
    components
        .iter()
        .enumerate()
        .map(|(index, component)| {
            let component = component.to_string_lossy();
            if index + 1 == components.len() {
                match Path::new(component.as_ref())
                    .extension()
                    .and_then(|extension| extension.to_str())
                {
                    Some(extension) => format!("<file>.{extension}"),
                    None => "<file>".to_string(),
                }
            } else if component.bytes().all(|byte| byte.is_ascii_digit()) {
                "<number>".to_string()
            } else {
                "<segment>".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}
