use token_tracing_widget_lib::sources::session_files::DiscoveryLimits;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::widget_settings::WidgetSettingsSnapshot;
use token_tracing_widget_lib::AppState;

fn limits() -> DiscoveryLimits {
    DiscoveryLimits::new(10, 10_000)
}

#[test]
fn widget_settings_update_is_persisted_and_reloaded() {
    let profile = tempfile::tempdir().expect("profile directory should be created");
    let database = tempfile::tempdir().expect("database directory should be created");
    let database_path = database.path().join("index.sqlite");
    let state = AppState::from_paths(profile.path().to_path_buf(), &database_path, limits())
        .expect("runtime should open");

    assert_eq!(
        state.widget_settings().unwrap(),
        WidgetSettingsSnapshot::defaults()
    );

    let settings =
        WidgetSettingsSnapshot::new(false, [(Provider::Claude, true), (Provider::Codex, false)]);
    state.update_widget_settings(settings.clone()).unwrap();
    assert_eq!(state.widget_settings().unwrap(), settings);

    drop(state);
    let reopened = AppState::from_paths(profile.path().to_path_buf(), &database_path, limits())
        .expect("restarted runtime should open");
    assert_eq!(reopened.widget_settings().unwrap(), settings);
}

#[test]
fn unavailable_runtime_cannot_read_or_write_widget_settings() {
    let state = AppState::unavailable();

    assert_eq!(
        state.widget_settings().unwrap_err().to_string(),
        "unavailable"
    );
    assert_eq!(
        state
            .update_widget_settings(WidgetSettingsSnapshot::defaults())
            .unwrap_err()
            .to_string(),
        "unavailable"
    );
}
