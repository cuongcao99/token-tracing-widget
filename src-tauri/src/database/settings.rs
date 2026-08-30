//! User settings table access.

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::sources::source_config::{
    parse_explicit_root, LoadedSourceConfigs, SourceConfig, SourceConfigSet,
};
use crate::types::provider::Provider;

fn key(provider: Provider, field: &str) -> String {
    format!("source.{}.{}", provider.as_str(), field)
}

pub(crate) fn load_source_configs(
    connection: &Connection,
) -> rusqlite::Result<LoadedSourceConfigs> {
    let mut configs = SourceConfigSet::defaults();
    let mut invalid_providers = Vec::new();

    for provider in [Provider::Claude, Provider::Codex] {
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

fn load_value(connection: &Connection, setting_key: &str) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT setting_value FROM settings WHERE setting_key = ?1",
            [setting_key],
            |row| row.get(0),
        )
        .optional()
}
