//! Persisted, privacy-safe presentation preferences.

use serde::Serialize;

use super::provider::Provider;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisibleProviderSetting {
    pub provider: Provider,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WidgetSettingsSnapshot {
    pub visible_providers: Vec<VisibleProviderSetting>,
    pub dark_mode: bool,
}

impl WidgetSettingsSnapshot {
    pub fn defaults() -> Self {
        Self::new(true, [(Provider::Claude, true), (Provider::Codex, true)])
    }

    pub fn new(dark_mode: bool, visible_providers: [(Provider, bool); 2]) -> Self {
        Self {
            visible_providers: visible_providers
                .into_iter()
                .map(|(provider, visible)| VisibleProviderSetting { provider, visible })
                .collect(),
            dark_mode,
        }
    }

    pub fn is_visible(&self, provider: Provider) -> bool {
        self.visible_providers
            .iter()
            .find(|entry| entry.provider == provider)
            .is_some_and(|entry| entry.visible)
    }

    pub fn dark_mode(&self) -> bool {
        self.dark_mode
    }
}

pub type WidgetSettings = WidgetSettingsSnapshot;
