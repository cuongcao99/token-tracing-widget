//! Typed application update commands for the settings window.

use tauri::{AppHandle, State};

use crate::app::runtime::AppState;
use crate::app::updates as update_service;
use crate::types::updates::UpdateCheckResult;

fn acquire_update_operation(
    state: &AppState,
) -> Result<update_service::UpdateOperationGuard, String> {
    state
        .try_begin_update()
        .ok_or_else(|| "update_busy".to_owned())
}

#[tauri::command]
pub(crate) async fn check_for_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UpdateCheckResult, String> {
    let _guard = acquire_update_operation(state.inner())?;
    update_service::check_for_update(&app)
        .await
        .map_err(|error| error.code().to_owned())
}

#[tauri::command]
pub(crate) async fn install_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _guard = acquire_update_operation(state.inner())?;
    update_service::install_update(&app)
        .await
        .map_err(|error| error.code().to_owned())
}

#[cfg(test)]
mod tests {
    use super::acquire_update_operation;
    use crate::app::runtime::AppState;

    #[test]
    fn update_commands_use_the_shared_operation_guard() {
        let state = AppState::unavailable();
        let first = acquire_update_operation(&state).unwrap();

        assert!(matches!(
            acquire_update_operation(&state),
            Err(error) if error == "update_busy"
        ));

        drop(first);
        assert!(acquire_update_operation(&state).is_ok());
    }
}
