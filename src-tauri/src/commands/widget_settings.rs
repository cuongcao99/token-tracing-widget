//! Typed widget presentation-preference command boundary.

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

use crate::app::runtime::{AppState, RuntimeError};
use crate::types::provider::Provider;
use crate::types::theme::Theme;
use crate::types::widget_settings::WidgetSettingsSnapshot;

pub const WIDGET_SETTINGS_CHANGED_EVENT: &str = "widget-settings-changed";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisibleProviderSettingInput {
    pub provider: Provider,
    pub visible: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WidgetSettingsInput {
    pub visible_providers: Vec<VisibleProviderSettingInput>,
    pub dark_mode: bool,
    #[serde(default)]
    pub theme: Theme,
}

impl WidgetSettingsInput {
    pub(crate) fn into_snapshot(self) -> Result<WidgetSettingsSnapshot, String> {
        if self.visible_providers.len() != Provider::all().len() {
            return Err("invalid_widget_settings".to_owned());
        }

        let mut visible_providers = Vec::with_capacity(self.visible_providers.len());

        for setting in self.visible_providers {
            if !Provider::all().contains(&setting.provider)
                || visible_providers
                    .iter()
                    .any(|(provider, _)| *provider == setting.provider)
            {
                return Err("invalid_widget_settings".to_owned());
            }
            visible_providers.push((setting.provider, setting.visible));
        }

        let ordered_visibility = Provider::all().iter().copied().map(|provider| {
            let visible = visible_providers
                .iter()
                .find(|(entry_provider, _)| *entry_provider == provider)
                .map(|(_, visible)| *visible)
                .unwrap_or(false);
            (provider, visible)
        });

        Ok(WidgetSettingsSnapshot::with_theme(
            self.theme,
            self.dark_mode,
            ordered_visibility,
        ))
    }
}

pub(crate) fn widget_settings_snapshot(state: &AppState) -> Result<WidgetSettingsSnapshot, String> {
    state.widget_settings().map_err(sanitize_runtime_error)
}

fn sanitize_runtime_error(error: RuntimeError) -> String {
    match error {
        RuntimeError::Unavailable | RuntimeError::StatePoisoned => {
            "widget_settings_unavailable".to_owned()
        }
        RuntimeError::Settings(_) => "widget_settings_write".to_owned(),
        RuntimeError::Collection(_) => "widget_settings_unavailable".to_owned(),
    }
}

#[tauri::command]
pub(crate) fn get_widget_settings(
    state: State<'_, AppState>,
) -> Result<WidgetSettingsSnapshot, String> {
    widget_settings_snapshot(state.inner())
}

#[tauri::command]
pub(crate) async fn update_widget_settings(
    state: State<'_, AppState>,
    app: AppHandle,
    settings: WidgetSettingsInput,
) -> Result<WidgetSettingsSnapshot, String> {
    let snapshot = settings.into_snapshot()?;
    let app_state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app_state.update_widget_settings(snapshot))
        .await
        .map_err(|_| "widget_settings_write".to_owned())?
        .map_err(sanitize_runtime_error)?;
    let snapshot = widget_settings_snapshot(state.inner())?;
    app.emit(WIDGET_SETTINGS_CHANGED_EVENT, &snapshot)
        .map_err(|_| "widget_settings_event".to_owned())?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::{widget_settings_snapshot, WidgetSettingsInput, WIDGET_SETTINGS_CHANGED_EVENT};
    use crate::app::runtime::AppState;
    use crate::types::provider::Provider;
    use crate::types::theme::Theme;
    use crate::types::widget_settings::WidgetSettingsSnapshot;

    #[test]
    fn input_builds_a_fixed_provider_snapshot() {
        let input: WidgetSettingsInput = serde_json::from_value(serde_json::json!({
            "visibleProviders": [
                { "provider": "claude", "visible": true },
                { "provider": "codex", "visible": false }
            ],
            "darkMode": false
        }))
        .unwrap();

        let snapshot = input.into_snapshot().unwrap();
        assert_eq!(
            snapshot,
            WidgetSettingsSnapshot::new(
                false,
                [(Provider::Claude, true), (Provider::Codex, false)]
            )
        );
        assert_eq!(snapshot.theme(), Theme::Claude);
    }

    #[test]
    fn input_rejects_duplicate_or_missing_providers() {
        for visible_providers in [
            serde_json::json!([
                { "provider": "claude", "visible": true },
                { "provider": "claude", "visible": false }
            ]),
            serde_json::json!([{ "provider": "claude", "visible": true }]),
        ] {
            let input: WidgetSettingsInput = serde_json::from_value(serde_json::json!({
                "visibleProviders": visible_providers,
                "darkMode": true
            }))
            .unwrap();

            assert_eq!(
                input.into_snapshot().unwrap_err(),
                "invalid_widget_settings"
            );
        }
    }

    #[test]
    fn unavailable_snapshot_is_sanitized_for_the_frontend() {
        assert_eq!(
            widget_settings_snapshot(&AppState::unavailable()).unwrap_err(),
            "widget_settings_unavailable"
        );
    }

    #[test]
    fn settings_event_name_is_stable() {
        assert_eq!(WIDGET_SETTINGS_CHANGED_EVENT, "widget-settings-changed");
    }
}
