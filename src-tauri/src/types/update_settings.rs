//! Persisted application-update preferences.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsSnapshot {
    pub auto_update: bool,
}

impl Default for UpdateSettingsSnapshot {
    fn default() -> Self {
        Self { auto_update: false }
    }
}
