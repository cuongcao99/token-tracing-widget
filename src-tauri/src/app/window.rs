//! Overlay and settings window setup.

use crate::app::tray::MAIN_WINDOW_LABEL;

fn should_hide_on_close(window_label: &str) -> bool {
    window_label == MAIN_WINDOW_LABEL
}

pub(crate) fn handle_window_event<R: tauri::Runtime>(
    window: &tauri::Window<R>,
    event: &tauri::WindowEvent,
) {
    if !should_hide_on_close(window.label()) {
        return;
    }

    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        if let Err(error) = window.hide() {
            eprintln!("shell:close_to_hide:{error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tray::MAIN_WINDOW_LABEL;

    #[test]
    fn only_main_window_uses_close_to_hide() {
        assert!(should_hide_on_close(MAIN_WINDOW_LABEL));
        assert!(!should_hide_on_close("settings"));
    }
}
