//! Rust-owned application update policy and updater integration.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

use crate::app::runtime::AppState;
use crate::types::updates::UpdateCheckResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpdateError {
    CheckFailed,
    NoUpdate,
    InstallFailed,
}

impl UpdateError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::CheckFailed => "update_check_failed",
            Self::NoUpdate => "update_unavailable",
            Self::InstallFailed => "update_install_failed",
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct UpdateOperationState(Arc<AtomicBool>);

pub(crate) struct UpdateOperationGuard(Arc<AtomicBool>);

impl UpdateOperationState {
    pub(crate) fn try_acquire(&self) -> Option<UpdateOperationGuard> {
        self.0
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| UpdateOperationGuard(self.0.clone()))
    }
}

impl Drop for UpdateOperationGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(crate) fn should_install_automatic_update(
    auto_update_enabled: bool,
    update_available: bool,
) -> bool {
    auto_update_enabled && update_available
}

pub(crate) async fn check_for_update(app: &AppHandle) -> Result<UpdateCheckResult, UpdateError> {
    let current_version = app.package_info().version.to_string();
    let updater = app
        .updater_builder()
        .build()
        .map_err(|_| UpdateError::CheckFailed)?;
    let available_version = updater
        .check()
        .await
        .map_err(|_| UpdateError::CheckFailed)?
        .map(|update| update.version);

    Ok(UpdateCheckResult {
        current_version,
        available_version,
    })
}

pub(crate) async fn install_update(app: &AppHandle) -> Result<(), UpdateError> {
    let updater = app
        .updater_builder()
        .build()
        .map_err(|_| UpdateError::InstallFailed)?;
    let update = updater
        .check()
        .await
        .map_err(|_| UpdateError::InstallFailed)?
        .ok_or(UpdateError::NoUpdate)?;

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|_| UpdateError::InstallFailed)?;

    #[cfg(not(windows))]
    app.restart();

    Ok(())
}

pub(crate) fn schedule_startup_update(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let enabled = state
            .update_settings()
            .map(|settings| settings.auto_update)
            .unwrap_or(false);
        if !enabled {
            return;
        }

        let Some(_guard) = state.try_begin_update() else {
            return;
        };

        let check = match check_for_update(&app).await {
            Ok(check) => check,
            Err(error) => {
                eprintln!("update:{}", error.code());
                return;
            }
        };
        if !should_install_automatic_update(enabled, check.available_version.is_some()) {
            return;
        }

        if let Err(error) = install_update(&app).await {
            eprintln!("update:{}", error.code());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{should_install_automatic_update, UpdateOperationState};

    #[test]
    fn automatic_install_requires_opt_in_and_an_available_update() {
        assert!(!should_install_automatic_update(false, true));
        assert!(!should_install_automatic_update(true, false));
        assert!(should_install_automatic_update(true, true));
    }

    #[test]
    fn update_operation_state_allows_only_one_operation() {
        let state = UpdateOperationState::default();
        let first = state.try_acquire();

        assert!(first.is_some());
        assert!(state.try_acquire().is_none());
        drop(first);
        assert!(state.try_acquire().is_some());
    }
}
