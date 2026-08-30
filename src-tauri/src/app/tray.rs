//! System-tray menu and actions.

use tauri::Manager;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
pub(crate) const TRAY_ID: &str = "main-tray";
pub(crate) const SHOW_MENU_ID: &str = "show";
pub(crate) const HIDE_MENU_ID: &str = "hide";
pub(crate) const QUIT_MENU_ID: &str = "quit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayAction {
    Show,
    Hide,
    Quit,
    Ignore,
}

pub(crate) fn action_for_menu_id(menu_id: &str) -> TrayAction {
    match menu_id {
        SHOW_MENU_ID => TrayAction::Show,
        HIDE_MENU_ID => TrayAction::Hide,
        QUIT_MENU_ID => TrayAction::Quit,
        _ => TrayAction::Ignore,
    }
}

fn menu_items() -> [(&'static str, &'static str); 3] {
    [
        (SHOW_MENU_ID, "Show"),
        (HIDE_MENU_ID, "Hide"),
        (QUIT_MENU_ID, "Quit"),
    ]
}

fn set_main_window_visibility<R: tauri::Runtime>(app: &tauri::AppHandle<R>, visible: bool) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    let result = if visible {
        window.show()
    } else {
        window.hide()
    };
    if let Err(error) = result {
        eprintln!("shell:window_visibility:{error}");
    }
}

pub(crate) fn setup_tray<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    let [(show_id, show_text), (hide_id, hide_text), (quit_id, quit_text)] = menu_items();
    let menu = tauri::menu::MenuBuilder::new(app)
        .text(show_id, show_text)
        .text(hide_id, hide_text)
        .separator()
        .text(quit_id, quit_text)
        .build()?;
    let icon = app.default_window_icon().cloned().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "default window icon unavailable",
        )
    })?;

    tauri::tray::TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .tooltip("Token Tracing")
        .on_menu_event(|app, event| match action_for_menu_id(event.id().as_ref()) {
            TrayAction::Show => set_main_window_visibility(app, true),
            TrayAction::Hide => set_main_window_visibility(app, false),
            TrayAction::Quit => app.exit(0),
            TrayAction::Ignore => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_menu_ids_map_to_exact_actions() {
        assert_eq!(action_for_menu_id(SHOW_MENU_ID), TrayAction::Show);
        assert_eq!(action_for_menu_id(HIDE_MENU_ID), TrayAction::Hide);
        assert_eq!(action_for_menu_id(QUIT_MENU_ID), TrayAction::Quit);
    }

    #[test]
    fn unknown_menu_ids_are_ignored() {
        assert_eq!(action_for_menu_id("settings"), TrayAction::Ignore);
        assert_eq!(action_for_menu_id(""), TrayAction::Ignore);
    }

    #[test]
    fn lifecycle_menu_ids_are_distinct() {
        assert_ne!(SHOW_MENU_ID, HIDE_MENU_ID);
        assert_ne!(SHOW_MENU_ID, QUIT_MENU_ID);
        assert_ne!(HIDE_MENU_ID, QUIT_MENU_ID);
    }

    #[test]
    fn lifecycle_menu_items_are_show_hide_quit_in_order() {
        assert_eq!(
            menu_items(),
            [
                (SHOW_MENU_ID, "Show"),
                (HIDE_MENU_ID, "Hide"),
                (QUIT_MENU_ID, "Quit"),
            ]
        );
    }
}
