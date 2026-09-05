//! Typed application-update preference commands for the settings window.

use serde::Deserialize;
use tauri::State;

use crate::app::runtime::{AppState, RuntimeError};
use crate::types::update_settings::UpdateSettingsSnapshot;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSettingsInput {
    pub auto_update: bool,
}

impl UpdateSettingsInput {
    fn into_snapshot(self) -> UpdateSettingsSnapshot {
        UpdateSettingsSnapshot {
            auto_update: self.auto_update,
        }
    }
}

fn update_settings_snapshot(state: &AppState) -> Result<UpdateSettingsSnapshot, String> {
    state.update_settings().map_err(sanitize_runtime_error)
}

fn sanitize_runtime_error(error: RuntimeError) -> String {
    match error {
        RuntimeError::Unavailable | RuntimeError::StatePoisoned => {
            "update_settings_unavailable".to_owned()
        }
        RuntimeError::Settings(_) => "update_settings_write".to_owned(),
        RuntimeError::Collection(_) => "update_settings_unavailable".to_owned(),
    }
}

#[tauri::command]
pub(crate) fn get_update_settings(
    state: State<'_, AppState>,
) -> Result<UpdateSettingsSnapshot, String> {
    update_settings_snapshot(state.inner())
}

#[tauri::command]
pub(crate) async fn save_update_settings(
    state: State<'_, AppState>,
    settings: UpdateSettingsInput,
) -> Result<UpdateSettingsSnapshot, String> {
    let snapshot = settings.into_snapshot();
    let app_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app_state.save_update_settings(snapshot))
        .await
        .map_err(|_| "update_settings_write".to_owned())?
        .map_err(sanitize_runtime_error)?;
    update_settings_snapshot(state.inner())
}

#[cfg(test)]
mod tests {
    use super::{update_settings_snapshot, UpdateSettingsInput};
    use crate::app::runtime::AppState;
    use crate::types::update_settings::UpdateSettingsSnapshot;

    #[test]
    fn input_maps_to_the_persisted_snapshot() {
        let input: UpdateSettingsInput = serde_json::from_value(serde_json::json!({
            "autoUpdate": true
        }))
        .unwrap();

        assert_eq!(
            input.into_snapshot(),
            UpdateSettingsSnapshot { auto_update: true }
        );
    }

    #[test]
    fn input_rejects_unknown_fields() {
        let value = serde_json::json!({
            "autoUpdate": true,
            "endpoint": "private"
        });

        assert!(serde_json::from_value::<UpdateSettingsInput>(value).is_err());
    }

    #[test]
    fn unavailable_snapshot_is_sanitized_for_the_frontend() {
        assert_eq!(
            update_settings_snapshot(&AppState::unavailable()).unwrap_err(),
            "update_settings_unavailable"
        );
    }
}
