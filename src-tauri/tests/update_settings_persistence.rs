use token_tracing_widget_lib::database::store::IndexStore;
use token_tracing_widget_lib::types::update_settings::UpdateSettingsSnapshot;

#[test]
fn automatic_update_preference_defaults_false_and_round_trips() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("index.sqlite");
    let mut store = IndexStore::open(&path).unwrap();

    assert_eq!(
        store.load_update_settings().unwrap(),
        UpdateSettingsSnapshot::default()
    );

    let settings = UpdateSettingsSnapshot { auto_update: true };
    store.save_update_settings(&settings).unwrap();
    assert_eq!(store.load_update_settings().unwrap(), settings);

    drop(store);
    let reopened = IndexStore::open(&path).unwrap();
    assert_eq!(reopened.load_update_settings().unwrap(), settings);
}

#[test]
fn invalid_automatic_update_value_falls_back_to_false() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("index.sqlite");
    let store = IndexStore::open(&path).unwrap();
    drop(store);

    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "INSERT INTO settings (setting_key, setting_value) VALUES ('update.auto_update', 'yes')",
            [],
        )
        .unwrap();
    drop(connection);

    let reopened = IndexStore::open(&path).unwrap();
    assert!(!reopened.load_update_settings().unwrap().auto_update);
}
