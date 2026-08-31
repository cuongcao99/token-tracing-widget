//! User settings table access.

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::sources::source_config::{
    parse_explicit_root, LoadedSourceConfigs, SourceConfig, SourceConfigSet,
};
use crate::types::provider::Provider;
use crate::types::theme::Theme;
use crate::types::widget_settings::WidgetSettingsSnapshot;

const WIDGET_DARK_MODE_KEY: &str = "widget.dark_mode";
const WIDGET_THEME_KEY: &str = "widget.theme";

fn key(provider: Provider, field: &str) -> String {
    format!("source.{}.{}", provider.as_str(), field)
}

pub(crate) fn load_source_configs(
    connection: &Connection,
) -> rusqlite::Result<LoadedSourceConfigs> {
    let mut configs = SourceConfigSet::defaults();
    let mut invalid_providers = Vec::new();

    for provider in Provider::all() {
        let provider = *provider;
        let enabled_value = load_value(connection, &key(provider, "enabled"))?;
        let root_value = load_value(connection, &key(provider, "root_override"))?;
        let mut invalid = false;

        let enabled = match enabled_value.as_deref() {
            None | Some("1") => true,
            Some("0") => false,
            Some(_) => {
                invalid = true;
                true
            }
        };
        let root_override = match root_value {
            None => None,
            Some(value) => match parse_explicit_root(&value) {
                Ok(path) => Some(path),
                Err(_) => {
                    invalid = true;
                    None
                }
            },
        };

        let config = SourceConfig::try_new(provider, enabled, root_override)
            .expect("settings parser should produce a valid source config");
        configs.replace(config);
        if invalid {
            invalid_providers.push(provider);
        }
    }

    Ok(LoadedSourceConfigs {
        configs,
        invalid_providers,
    })
}

pub(crate) fn save_source_config(
    transaction: &Transaction<'_>,
    config: &SourceConfig,
) -> rusqlite::Result<()> {
    transaction.execute(
        r#"
        INSERT INTO settings (setting_key, setting_value)
        VALUES (?1, ?2)
        ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value
        "#,
        params![
            key(config.provider(), "enabled"),
            if config.enabled() { "1" } else { "0" },
        ],
    )?;

    let root_key = key(config.provider(), "root_override");
    if let Some(root) = config.root_override() {
        transaction.execute(
            r#"
            INSERT INTO settings (setting_key, setting_value)
            VALUES (?1, ?2)
            ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value
            "#,
            params![root_key, root.to_string_lossy().as_ref()],
        )?;
    } else {
        transaction.execute(
            "DELETE FROM settings WHERE setting_key = ?1",
            params![root_key],
        )?;
    }

    Ok(())
}

pub(crate) fn load_widget_settings(
    connection: &Connection,
) -> rusqlite::Result<WidgetSettingsSnapshot> {
    let dark_mode = load_bool_value(connection, WIDGET_DARK_MODE_KEY, true)?;
    let theme = load_value(connection, WIDGET_THEME_KEY)?
        .as_deref()
        .and_then(Theme::from_str)
        .unwrap_or_default();
    let visible_providers = Provider::all()
        .iter()
        .copied()
        .map(|provider| {
            load_bool_value(connection, &widget_visible_key(provider), true)
                .map(|visible| (provider, visible))
        })
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(WidgetSettingsSnapshot::with_theme(
        theme,
        dark_mode,
        visible_providers,
    ))
}

pub(crate) fn save_widget_settings(
    transaction: &Transaction<'_>,
    settings: &WidgetSettingsSnapshot,
) -> rusqlite::Result<()> {
    save_value(
        transaction,
        WIDGET_DARK_MODE_KEY,
        if settings.dark_mode() { "1" } else { "0" },
    )?;
    save_value(transaction, WIDGET_THEME_KEY, settings.theme().as_str())?;
    for provider in Provider::all() {
        save_value(
            transaction,
            &widget_visible_key(*provider),
            if settings.is_visible(*provider) {
                "1"
            } else {
                "0"
            },
        )?;
    }
    Ok(())
}

fn widget_visible_key(provider: Provider) -> String {
    format!("widget.visible.{}", provider.as_str())
}

fn load_value(connection: &Connection, setting_key: &str) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT setting_value FROM settings WHERE setting_key = ?1",
            [setting_key],
            |row| row.get(0),
        )
        .optional()
}

fn load_bool_value(
    connection: &Connection,
    setting_key: &str,
    default: bool,
) -> rusqlite::Result<bool> {
    Ok(match load_value(connection, setting_key)?.as_deref() {
        Some("1") => true,
        Some("0") => false,
        _ => default,
    })
}

fn save_value(
    transaction: &Transaction<'_>,
    setting_key: &str,
    setting_value: &str,
) -> rusqlite::Result<()> {
    transaction.execute(
        r#"
        INSERT INTO settings (setting_key, setting_value)
        VALUES (?1, ?2)
        ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value
        "#,
        params![setting_key, setting_value],
    )?;
    Ok(())
}
