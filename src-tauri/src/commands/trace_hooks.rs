//! Explicit user-consent commands for provider lifecycle hook configuration.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::hooks_config::{
    has_provider_hooks, merge_provider_hooks, remove_provider_hooks, HookConfigError, HookProvider,
};
use crate::types::provider::Provider;
use crate::types::trace_hooks::{TraceHookState, TraceHookStatus, TraceHooksSnapshot};

const CLAUDE_CONFIG_DIRECTORY: &str = ".claude";
const CLAUDE_CONFIG_FILE: &str = "settings.json";
const CODEX_CONFIG_DIRECTORY: &str = ".codex";
const CODEX_CONFIG_FILE: &str = "hooks.json";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TraceHookSettingsInput {
    pub provider: Provider,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct HookConfigPaths {
    claude: PathBuf,
    codex: PathBuf,
}

impl HookConfigPaths {
    fn from_profile_root(profile_root: impl Into<PathBuf>) -> Self {
        let profile_root = profile_root.into();
        Self {
            claude: profile_root
                .join(CLAUDE_CONFIG_DIRECTORY)
                .join(CLAUDE_CONFIG_FILE),
            codex: profile_root
                .join(CODEX_CONFIG_DIRECTORY)
                .join(CODEX_CONFIG_FILE),
        }
    }

    fn from_user_profile() -> Result<Self, &'static str> {
        let profile_root = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .ok_or("hook_profile_unavailable")?;
        Ok(Self::from_profile_root(profile_root))
    }

    fn path(&self, provider: Provider) -> &Path {
        match provider {
            Provider::Claude => &self.claude,
            Provider::Codex => &self.codex,
        }
    }
}

#[tauri::command]
pub(crate) fn get_trace_hook_status() -> Result<TraceHooksSnapshot, String> {
    let paths = HookConfigPaths::from_user_profile().map_err(str::to_owned)?;
    trace_hook_snapshot(&paths)
}

#[tauri::command]
pub(crate) fn update_trace_hook(
    settings: TraceHookSettingsInput,
) -> Result<TraceHooksSnapshot, String> {
    let paths = HookConfigPaths::from_user_profile().map_err(str::to_owned)?;
    update_trace_hook_for(
        &paths,
        settings.provider,
        settings.enabled,
        &hook_command(settings.provider)?,
    )?;
    trace_hook_snapshot(&paths)
}

fn update_trace_hook_for(
    paths: &HookConfigPaths,
    provider: Provider,
    enabled: bool,
    command: &str,
) -> Result<(), String> {
    let path = paths.path(provider);
    let existing = read_config(path)?;
    let Some(existing_config) = existing.clone() else {
        if !enabled {
            return Ok(());
        }
        let next = transform_config(json!({}), provider, enabled, command)?;
        return write_config(path, &next);
    };

    let next = transform_config(existing_config.clone(), provider, enabled, command)?;
    if next != existing_config {
        write_config(path, &next)?;
    }
    Ok(())
}

fn trace_hook_snapshot(paths: &HookConfigPaths) -> Result<TraceHooksSnapshot, String> {
    Provider::all()
        .iter()
        .copied()
        .map(|provider| trace_hook_status(paths, provider))
        .collect::<Result<Vec<_>, _>>()
        .map(|providers| TraceHooksSnapshot { providers })
}

fn trace_hook_status(
    paths: &HookConfigPaths,
    provider: Provider,
) -> Result<TraceHookStatus, String> {
    let Some(config) = read_config(paths.path(provider))? else {
        return Ok(not_installed_status(provider));
    };
    let configured = has_provider_hooks(&config, hook_provider(provider), &hook_command(provider)?)
        .map_err(sanitize_config_error)?;

    Ok(if configured {
        TraceHookStatus {
            provider,
            state: TraceHookState::Configured,
            requires_trust: provider == Provider::Codex,
        }
    } else {
        not_installed_status(provider)
    })
}

fn not_installed_status(provider: Provider) -> TraceHookStatus {
    TraceHookStatus {
        provider,
        state: TraceHookState::NotInstalled,
        requires_trust: false,
    }
}

fn transform_config(
    config: Value,
    provider: Provider,
    enabled: bool,
    command: &str,
) -> Result<Value, String> {
    let provider = hook_provider(provider);
    if enabled {
        merge_provider_hooks(config, provider, command).map_err(sanitize_config_error)
    } else {
        remove_provider_hooks(config, provider, command).map_err(sanitize_config_error)
    }
}

fn read_config(path: &Path) -> Result<Option<Value>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("hook_config_read".to_owned()),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "hook_config_invalid".to_owned())
}

fn write_config(path: &Path, config: &Value) -> Result<(), String> {
    let parent = path.parent().ok_or("hook_config_write")?;
    fs::create_dir_all(parent).map_err(|_| "hook_config_write")?;

    let mut payload = serde_json::to_vec_pretty(config).map_err(|_| "hook_config_write")?;
    payload.push(b'\n');

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("hooks");
    let temporary_path = parent.join(format!(".{file_name}.{}.{}.tmp", process::id(), nonce));

    let result = write_temporary_file(&temporary_path, &payload)
        .and_then(|_| replace_file(&temporary_path, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result.map_err(|_| "hook_config_write".to_owned())
}

fn write_temporary_file(path: &Path, payload: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(payload)?;
    file.sync_all()
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing_name: *const u16, new_name: *const u16, flags: u32) -> i32;
    }

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let flags = 0x0000_0001_u32 | 0x0000_0008_u32;
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), flags) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

fn hook_command(provider: Provider) -> Result<String, String> {
    let executable = std::env::current_exe().map_err(|_| "hook_command_unavailable")?;
    Ok(format!(
        "\"{}\" --hook {}",
        executable.to_string_lossy(),
        provider.as_str()
    ))
}

fn hook_provider(provider: Provider) -> HookProvider {
    match provider {
        Provider::Claude => HookProvider::Claude,
        Provider::Codex => HookProvider::Codex,
    }
}

fn sanitize_config_error(error: HookConfigError) -> String {
    match error {
        HookConfigError::EmptyCommand => "hook_command_unavailable".to_owned(),
        HookConfigError::RootMustBeObject | HookConfigError::InvalidShape { .. } => {
            "hook_config_invalid".to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::{update_trace_hook_for, HookConfigPaths, TraceHookSettingsInput};
    use crate::types::provider::Provider;
    use crate::types::trace_hooks::TraceHookState;

    const CLAUDE_COMMAND: &str = "widget.exe --hook claude";
    const CODEX_COMMAND: &str = "widget.exe --hook codex";

    #[test]
    fn install_and_remove_preserve_unrelated_user_configuration() {
        let profile = tempfile::tempdir().unwrap();
        let paths = HookConfigPaths::from_profile_root(profile.path());
        let config_path = paths.path(Provider::Claude);
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        let original = json!({
            "permissions": {"allow": ["Bash"]},
            "hooks": {"PreToolUse": [{"hooks": [{"type": "command", "command": "keep"}]}]}
        });
        fs::write(config_path, serde_json::to_vec(&original).unwrap()).unwrap();

        update_trace_hook_for(&paths, Provider::Claude, true, CLAUDE_COMMAND).unwrap();
        let installed: serde_json::Value =
            serde_json::from_slice(&fs::read(config_path).unwrap()).unwrap();
        assert_eq!(installed["permissions"], original["permissions"]);
        assert!(installed["hooks"]["SessionEnd"].is_array());

        update_trace_hook_for(&paths, Provider::Claude, false, CLAUDE_COMMAND).unwrap();
        let removed: serde_json::Value =
            serde_json::from_slice(&fs::read(config_path).unwrap()).unwrap();
        assert_eq!(removed, original);
    }

    #[test]
    fn status_is_provider_specific_and_marks_codex_trust_as_provider_owned() {
        let profile = tempfile::tempdir().unwrap();
        let paths = HookConfigPaths::from_profile_root(profile.path());
        update_trace_hook_for(&paths, Provider::Codex, true, CODEX_COMMAND).unwrap();

        let snapshot = trace_hook_snapshot_with_commands(&paths);
        let claude = snapshot
            .providers
            .iter()
            .find(|status| status.provider == Provider::Claude)
            .unwrap();
        let codex = snapshot
            .providers
            .iter()
            .find(|status| status.provider == Provider::Codex)
            .unwrap();
        assert_eq!(claude.state, TraceHookState::NotInstalled);
        assert_eq!(codex.state, TraceHookState::Configured);
        assert!(codex.requires_trust);
    }

    #[test]
    fn malformed_config_is_not_overwritten() {
        let profile = tempfile::tempdir().unwrap();
        let paths = HookConfigPaths::from_profile_root(profile.path());
        let config_path = paths.path(Provider::Claude);
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        let malformed = b"{ not json";
        fs::write(config_path, malformed).unwrap();

        let error =
            update_trace_hook_for(&paths, Provider::Claude, true, CLAUDE_COMMAND).unwrap_err();
        assert_eq!(error, "hook_config_invalid");
        assert_eq!(fs::read(config_path).unwrap(), malformed);
    }

    #[test]
    fn input_rejects_unknown_fields() {
        let input = serde_json::from_value::<TraceHookSettingsInput>(json!({
            "provider": "claude",
            "enabled": true,
            "prompt": "private"
        }));
        assert!(input.is_err());
    }

    fn trace_hook_snapshot_with_commands(paths: &HookConfigPaths) -> super::TraceHooksSnapshot {
        let mut providers = Vec::new();
        for provider in Provider::all() {
            let command = match provider {
                Provider::Claude => CLAUDE_COMMAND,
                Provider::Codex => CODEX_COMMAND,
            };
            let config = super::read_config(paths.path(*provider)).unwrap();
            let configured = config
                .as_ref()
                .map(|config| {
                    crate::hooks_config::has_provider_hooks(
                        config,
                        super::hook_provider(*provider),
                        command,
                    )
                    .unwrap()
                })
                .unwrap_or(false);
            providers.push(if configured {
                crate::types::trace_hooks::TraceHookStatus {
                    provider: *provider,
                    state: TraceHookState::Configured,
                    requires_trust: *provider == Provider::Codex,
                }
            } else {
                super::not_installed_status(*provider)
            });
        }
        super::TraceHooksSnapshot { providers }
    }
}
