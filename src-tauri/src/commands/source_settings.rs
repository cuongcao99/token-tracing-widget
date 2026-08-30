//! Typed source configuration commands for the settings window.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app::live_collection::{update_source_config_and_refresh, LiveCollectionHandle};
use crate::app::runtime::{AppState, RuntimeError};
use crate::sources::source_config::{parse_explicit_root, SourceConfig};
use crate::types::provider::Provider;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSettingsInput {
    pub provider: Provider,
    pub enabled: bool,
    pub root_override: Option<String>,
}

impl SourceSettingsInput {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn into_config(self) -> Result<SourceConfig, String> {
        let root_override = self
            .root_override
            .filter(|root| !root.trim().is_empty())
            .map(|root| {
                parse_explicit_root(root.trim()).map_err(|error| format!("invalid_root:{error}"))
            })
            .transpose()?;

        SourceConfig::try_new(self.provider, self.enabled, root_override)
            .map_err(|error| format!("invalid_root:{error}"))
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceSettingsView {
    pub provider: Provider,
    pub enabled: bool,
    pub root_override: Option<String>,
}

impl From<SourceConfig> for SourceSettingsView {
    fn from(config: SourceConfig) -> Self {
        Self {
            provider: config.provider(),
            enabled: config.enabled(),
            root_override: config
                .root_override()
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
    [Provider::Claude, Provider::Codex]
        .into_iter()
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
pub(crate) fn update_source_settings(
    state: State<'_, AppState>,
    live_handle: State<'_, LiveCollectionHandle>,
    settings: SourceSettingsInput,
) -> Result<SourceSettingsSnapshot, String> {
    let config = settings.into_config()?;
    update_source_config_and_refresh(state.inner(), live_handle.inner(), config)
        .map_err(sanitize_runtime_error)?;
    source_settings_snapshot(state.inner())
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
                root_override: Some(
                    r"\\wsl.localhost\Ubuntu\home\user\.claude\projects".to_owned(),
                ),
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
            ["enabled", "provider", "rootOverride"]
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
            "rootOverride": null,
            "prompt": "private text"
        });

        assert!(serde_json::from_value::<SourceSettingsInput>(value).is_err());
    }

    #[test]
    fn blank_root_override_becomes_automatic_without_echoing_input() {
        let input = SourceSettingsInput {
            provider: Provider::Codex,
            enabled: true,
            root_override: Some("  ".to_owned()),
        };

        let config = input.into_config().expect("blank should mean automatic");
        assert!(config.root_override().is_none());
    }

    #[test]
    fn invalid_root_error_does_not_include_submitted_path() {
        let submitted = r"\\server\private\sessions";
        let input = SourceSettingsInput {
            provider: Provider::Claude,
            enabled: true,
            root_override: Some(submitted.to_owned()),
        };

        let error = input.into_config().unwrap_err();
        assert!(!error.contains(submitted));
        assert_eq!(error, "invalid_root:unsupported_unc");
    }
}
