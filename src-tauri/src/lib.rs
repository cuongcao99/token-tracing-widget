use serde::Serialize;
use tauri::Manager;

mod app;
pub mod collection;
pub mod commands;
pub mod database;
pub mod providers;
pub mod sources;
pub mod types;
pub mod usage;
pub mod utils;

pub use app::runtime::AppState;
pub use types::source_health::SourceHealth;
pub use types::usage_summary::UsageSummary;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UsageState {
    Loading,
    Active,
    Idle,
    Unavailable,
    Stale,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(app::window::handle_window_event)
        .setup(|app| {
            app::tray::setup_tray(app.handle())?;
            let state = app::runtime::initialize_from_app(app.handle());
            app.manage(state.clone());

            let managed = app.state::<app::runtime::AppState>();
            if let Ok(report) = managed.collect_once(&collection::WindowsClock::current()) {
                if commands::usage_summary::emit_usage_summary(app.handle(), &report.summary)
                    .is_err()
                {
                    eprintln!("summary_event:emit");
                }
            }

            let live_handle =
                app::live_collection::start_live_collection(state, app.handle().clone());
            app.manage(live_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::source_settings::get_source_settings,
            commands::source_settings::update_source_settings,
            commands::usage_summary::get_usage_summary
        ])
        .build(tauri::generate_context!())
        .expect("error while building token tracing widget")
        .run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app_handle
                    .state::<app::live_collection::LiveCollectionHandle>()
                    .shutdown();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_summary_contains_no_provider_data() {
        let summary = UsageSummary::unavailable();

        assert_eq!(summary.state, UsageState::Unavailable);
        assert_eq!(summary.today_tokens, 0);
        assert!(summary.provider.is_none());
        assert!(summary.current_session_tokens.is_none());
        assert!(summary.last_updated_at.is_none());
        assert!(summary.source_health.is_empty());

        let serialized = serde_json::to_value(&summary).expect("summary should serialize");
        let object = serialized
            .as_object()
            .expect("summary should serialize as an object");

        assert!(!object.contains_key("provider"));
        assert!(!object.contains_key("currentSessionTokens"));
        assert!(!object.contains_key("lastUpdatedAt"));
    }
}
