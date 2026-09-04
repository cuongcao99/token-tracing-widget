//! User settings table access.

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::sources::source_config::{
    is_wsl_path, parse_explicit_root, parse_windows_root, parse_wsl_root, LoadedSourceConfigs,
    SourceConfig, SourceConfigSet,
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
        let windows_root_value = load_value(connection, &key(provider, "windows_root_override"))?;
        let wsl_root_value = load_value(connection, &key(provider, "wsl_root_override"))?;
        let legacy_root_value = load_value(connection, &key(provider, "root_override"))?;
        let mut invalid = false;

        let enabled = match enabled_value.as_deref() {
            None | Some("1") => true,
            Some("0") => false,
            Some(_) => {
                invalid = true;
                true
            }
        };
        let (windows_root_override, wsl_root_override) =
            if windows_root_value.is_none() && wsl_root_value.is_none() {
                match legacy_root_value {
                    None => (None, None),
                    Some(value) => match parse_explicit_root(&value) {
                        Ok(path) if is_wsl_path(&path) => (None, Some(path)),
                        Ok(path) => (Some(path), None),
                        Err(_) => {
                            invalid = true;
                            (None, None)
                        }
                    },
                }
            } else {
                (
                    parse_root_value(windows_root_value, parse_windows_root, &mut invalid),
                    parse_root_value(wsl_root_value, parse_wsl_root, &mut invalid),
                )
            };

        let config = SourceConfig::try_new_with_roots(
            provider,
            enabled,
            windows_root_override,
            wsl_root_override,
        )
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

    save_optional_root(
        transaction,
        &key(config.provider(), "windows_root_override"),
        config.windows_root_override(),
    )?;
    save_optional_root(
        transaction,
        &key(config.provider(), "wsl_root_override"),
        config.wsl_root_override(),
    )?;
    transaction.execute(
        "DELETE FROM settings WHERE setting_key = ?1",
        params![key(config.provider(), "root_override")],
    )?;

    Ok(())
}

fn parse_root_value(
    value: Option<String>,
    parser: fn(
        &str,
    ) -> Result<std::path::PathBuf, crate::sources::source_config::SourceConfigError>,
    invalid: &mut bool,
) -> Option<std::path::PathBuf> {
    match value {
        None => None,
        Some(value) => match parser(&value) {
            Ok(path) => Some(path),
            Err(_) => {
                *invalid = true;
                None
            }
        },
    }
}

fn save_optional_root(
    transaction: &Transaction<'_>,
    setting_key: &str,
    root: Option<&std::path::Path>,
) -> rusqlite::Result<()> {
    match root {
        Some(root) => save_value(transaction, setting_key, &root.to_string_lossy()),
        None => transaction
            .execute("DELETE FROM settings WHERE setting_key = ?1", [setting_key])
            .map(|_| ()),
    }
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
