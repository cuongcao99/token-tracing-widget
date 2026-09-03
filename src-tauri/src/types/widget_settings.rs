//! Persisted, privacy-safe presentation preferences.

use serde::Serialize;

use super::provider::Provider;
use super::theme::Theme;

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
    pub theme: Theme,
}

impl WidgetSettingsSnapshot {
    pub fn defaults() -> Self {
        Self::new(
            true,
            Provider::all()
                .iter()
                .copied()
                .map(|provider| (provider, true)),
        )
    }

    pub fn new<I>(dark_mode: bool, visible_providers: I) -> Self
    where
        I: IntoIterator<Item = (Provider, bool)>,
    {
        Self::with_theme(Theme::Claude, dark_mode, visible_providers)
    }

    pub fn with_theme<I>(theme: Theme, dark_mode: bool, visible_providers: I) -> Self
    where
        I: IntoIterator<Item = (Provider, bool)>,
    {
        Self {
            visible_providers: visible_providers
                .into_iter()
                .map(|(provider, visible)| VisibleProviderSetting { provider, visible })
                .collect(),
            dark_mode,
            theme,
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

    pub fn theme(&self) -> Theme {
        self.theme
    }
}
