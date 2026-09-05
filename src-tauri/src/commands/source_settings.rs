//! Typed source configuration commands for the settings window.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app::folder_picker::pick_folder;
use crate::app::live_collection::{update_source_config_and_refresh, LiveCollectionHandle};
use crate::app::runtime::{AppState, RuntimeError};
use crate::sources::source_config::{
    parse_windows_root, parse_wsl_root, SourceConfig, SourcePlatform,
};
use crate::types::provider::Provider;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSettingsInput {
    pub provider: Provider,
    pub enabled: bool,
    pub windows_root: Option<String>,
    pub wsl_root: Option<String>,
}

impl SourceSettingsInput {
    pub(crate) fn into_config(self) -> Result<SourceConfig, String> {
        let windows_root = parse_optional_root(self.windows_root, parse_windows_root)?;
        let wsl_root = parse_optional_root(self.wsl_root, parse_wsl_root)?;
        SourceConfig::try_new_with_roots(self.provider, self.enabled, windows_root, wsl_root)
            .map_err(|error| format!("invalid_root:{error}"))
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceSettingsView {
    pub provider: Provider,
    pub enabled: bool,
    pub windows_root: Option<String>,
    pub wsl_root: Option<String>,
}

impl From<SourceConfig> for SourceSettingsView {
    fn from(config: SourceConfig) -> Self {
        Self {
            provider: config.provider(),
            enabled: config.enabled(),
            windows_root: config
                .windows_root_override()
                .map(|path| path.to_string_lossy().into_owned()),
            wsl_root: config
                .wsl_root_override()
                .map(|path| path.to_string_lossy().into_owned()),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceSettingsSnapshot {
    pub sources: Vec<SourceSettingsView>,
}

pub(crate) fn source_settings_snapshot(state: &AppState) -> Result<SourceSettingsSnapshot, String> {
    Provider::all()
        .iter()
        .copied()
        .map(|provider| {
            state
                .source_config(provider)
                .map(SourceSettingsView::from)
                .map_err(sanitize_runtime_error)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|sources| SourceSettingsSnapshot { sources })
}

fn sanitize_runtime_error(error: RuntimeError) -> String {
    match error {
        RuntimeError::Unavailable | RuntimeError::StatePoisoned => {
            "settings_unavailable".to_owned()
        }
        RuntimeError::Settings(_) => "settings_write".to_owned(),
        RuntimeError::Collection(_) => "settings_refresh".to_owned(),
    }
}

#[tauri::command]
pub(crate) fn get_source_settings(
    state: State<'_, AppState>,
) -> Result<SourceSettingsSnapshot, String> {
    source_settings_snapshot(state.inner())
}

#[tauri::command]
pub(crate) async fn update_source_settings(
    state: State<'_, AppState>,
    live_handle: State<'_, LiveCollectionHandle>,
    settings: SourceSettingsInput,
) -> Result<SourceSettingsSnapshot, String> {
    let config = settings.into_config()?;
    let app_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app_state.update_source_config(config))
        .await
        .map_err(|_| "settings_write".to_owned())?
        .map_err(sanitize_runtime_error)?;
    let _ = live_handle.request_source_refresh();
    source_settings_snapshot(state.inner())
}

#[tauri::command]
pub(crate) async fn pick_source_root(
    state: State<'_, AppState>,
    live_handle: State<'_, LiveCollectionHandle>,
    provider: Provider,
    platform: SourcePlatform,
) -> Result<Option<SourceSettingsSnapshot>, String> {
    let existing_config = state
        .source_config(provider)
        .map_err(sanitize_runtime_error)?;
    let initial_path: Option<PathBuf> = state
        .source_root_path_for(provider, platform)
        .ok()
        .filter(|path| path.is_dir());
    let title = format!(
        "Choose {} {} source folder",
        provider.display_name(),
        match platform {
            SourcePlatform::Windows => "Windows",
            SourcePlatform::Wsl => "WSL",
        }
    );
    let selected_path =
        tauri::async_runtime::spawn_blocking(move || pick_folder(&title, initial_path.as_deref()))
            .await
            .map_err(|_| "source_root_open".to_owned())??;
    let Some(selected_path) = selected_path else {
        return Ok(None);
    };

    let config = existing_config
        .with_root_override(platform, Some(selected_path))
        .map_err(|_| "source_root_invalid".to_owned())?;
    update_source_config_and_refresh(state.inner(), live_handle.inner(), config)
        .map_err(sanitize_runtime_error)?;
    source_settings_snapshot(state.inner()).map(Some)
}

fn parse_optional_root(
    value: Option<String>,
    parser: fn(&str) -> Result<PathBuf, crate::sources::source_config::SourceConfigError>,
) -> Result<Option<PathBuf>, String> {
    value
        .filter(|root| !root.trim().is_empty())
        .map(|root| parser(root.trim()).map_err(|error| format!("invalid_root:{error}")))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::{SourceSettingsInput, SourceSettingsSnapshot, SourceSettingsView};
    use crate::types::provider::Provider;

    #[test]
    fn settings_snapshot_contains_only_allowed_fields() {
        let snapshot = SourceSettingsSnapshot {
            sources: vec![SourceSettingsView {
                provider: Provider::Claude,
                enabled: true,
                windows_root: Some(r"C:\Users\user\.claude\projects".to_owned()),
                wsl_root: Some(r"\\wsl.localhost\Ubuntu\home\user\.claude\projects".to_owned()),
            }],
        };
        let object = serde_json::to_value(snapshot)
            .expect("settings should serialize")
            .as_object()
            .cloned()
            .expect("settings should be an object");

        assert_eq!(
            object.keys().map(String::as_str).collect::<Vec<_>>(),
            ["sources"]
        );
        let source = object["sources"][0].as_object().unwrap();
        assert_eq!(
            source.keys().map(String::as_str).collect::<Vec<_>>(),
            ["enabled", "provider", "windowsRoot", "wslRoot"]
        );
        assert!(!serde_json::to_string(&object)
            .unwrap()
            .contains("profileRoot"));
        assert!(!serde_json::to_string(&object)
            .unwrap()
            .contains("rawRecord"));
    }

    #[test]
    fn input_rejects_unknown_raw_data_fields() {
        let value = serde_json::json!({
            "provider": "claude",
            "enabled": true,
            "windowsRoot": null,
            "wslRoot": null,
            "prompt": "private text"
        });

        assert!(serde_json::from_value::<SourceSettingsInput>(value).is_err());
    }

    #[test]
    fn blank_root_override_becomes_automatic_without_echoing_input() {
        let input = SourceSettingsInput {
            provider: Provider::Codex,
            enabled: true,
            windows_root: Some("  ".to_owned()),
            wsl_root: None,
        };

        let config = input.into_config().expect("blank should mean automatic");
        assert!(config.windows_root_override().is_none());
        assert!(config.wsl_root_override().is_none());
    }

    #[cfg(windows)]
    #[test]
    fn input_keeps_windows_and_wsl_roots_in_separate_slots() {
        let input = SourceSettingsInput {
            provider: Provider::Claude,
            enabled: true,
            windows_root: Some(r"C:\Users\user\.claude\projects".to_owned()),
            wsl_root: Some(r"\\wsl.localhost\Ubuntu\home\user\.claude\projects".to_owned()),
        };

        let config = input.into_config().expect("both roots should be accepted");
        assert!(config.windows_root_override().is_some());
        assert!(config.wsl_root_override().is_some());
    }

    #[test]
    fn invalid_root_error_does_not_include_submitted_path() {
        let submitted = r"\\server\private\sessions";
        let input = SourceSettingsInput {
            provider: Provider::Claude,
            enabled: true,
            windows_root: Some(submitted.to_owned()),
            wsl_root: None,
        };

        let error = input.into_config().unwrap_err();
        assert!(!error.contains(submitted));
        assert_eq!(error, "invalid_root:unsupported_unc");
    }
}
