use token_tracing_widget_lib::database::store::IndexStore;
use token_tracing_widget_lib::types::provider::Provider;
use token_tracing_widget_lib::types::widget_settings::WidgetSettingsSnapshot;

#[test]
fn widget_preferences_default_and_round_trip_without_schema_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("index.sqlite");
    let mut store = IndexStore::open(&path).unwrap();

    assert_eq!(
        store.load_widget_settings().unwrap(),
        WidgetSettingsSnapshot::defaults()
    );

    let settings =
        WidgetSettingsSnapshot::new(false, [(Provider::Claude, true), (Provider::Codex, false)]);
    store.save_widget_settings(&settings).unwrap();

    assert_eq!(store.load_widget_settings().unwrap(), settings);

    let connection = rusqlite::Connection::open(path).unwrap();
    let theme: String = connection
        .query_row(
            "SELECT setting_value FROM settings WHERE setting_key = 'widget.theme'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(theme, "claude");
}
