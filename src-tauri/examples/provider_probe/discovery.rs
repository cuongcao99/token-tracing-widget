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
                max_bytes: 10,
                max_records: 50_000,
                max_record_bytes: 1_048_576,
            },
        );
        assert_eq!(result.candidates.len(), 5);
        assert!(result.selected_bytes <= 10);
        assert!(result
            .candidates
            .iter()
            .all(|candidate| !candidate.layout_pattern.contains("session-")));
    }
}
use std::fs;
use std::path::{Path, PathBuf};
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
    pub size: u64,
}

pub struct DiscoveryResult {
    pub provider: Provider,
    pub root_state: RootState,
    pub candidates: Vec<CandidateFile>,
    pub selected_bytes: u64,
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
    let metadata = match fs::metadata(&root) {
        Ok(metadata) if metadata.is_dir() => metadata,
        Ok(_) => return empty_result(provider, RootState::Error),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return empty_result(provider, RootState::NotDetected);
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            return empty_result(provider, RootState::PermissionDenied);
        }
        Err(_) => return empty_result(provider, RootState::Error),
    };
    let _ = metadata;
    if let Err(error) = fs::read_dir(&root) {
        let state = match error.kind() {
            std::io::ErrorKind::NotFound => RootState::NotDetected,
            std::io::ErrorKind::PermissionDenied => RootState::PermissionDenied,
            _ => RootState::Error,
        };
        return empty_result(provider, state);
    }

    let mut discovered = Vec::new();
    collect_files(&root, &root, &mut discovered);
    discovered.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| left.layout_pattern.cmp(&right.layout_pattern))
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut candidates = Vec::new();
    let mut selected_bytes = 0_u64;
    for discovered in discovered.into_iter().take(limits.max_files) {
        if candidates.len() >= limits.max_files {
            break;
        }
        // The candidate list is bounded by file count. Inspection applies the
        // byte budget to actual reads; this field is a bounded accounting value.
        selected_bytes = selected_bytes
            .saturating_add(discovered.size)
            .min(limits.max_bytes);
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
    }
}

fn empty_result(provider: Provider, root_state: RootState) -> DiscoveryResult {
    DiscoveryResult {
        provider,
        root_state,
        candidates: Vec::new(),
        selected_bytes: 0,
    }
}

struct DiscoveredFile {
    path: PathBuf,
    layout_pattern: String,
    size: u64,
    modified: SystemTime,
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<DiscoveredFile>) {
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_files(root, &path, files);
            continue;
        }
        if !file_type.is_file() || !is_supported_extension(&path) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let layout_pattern = layout_pattern(relative);
        files.push(DiscoveredFile {
            path,
            layout_pattern,
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(UNIX_EPOCH),
        });
    }
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
