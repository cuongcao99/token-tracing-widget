//! System-tray menu and actions.

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
}
