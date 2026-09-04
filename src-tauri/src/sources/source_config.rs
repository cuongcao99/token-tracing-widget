//! Persisted provider source selection and explicit-root input validation.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::types::provider::Provider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceConfig {
    provider: Provider,
    enabled: bool,
    windows_root_override: Option<PathBuf>,
    wsl_root_override: Option<PathBuf>,
}

impl SourceConfig {
    pub fn try_new(
        provider: Provider,
        enabled: bool,
        root_override: Option<PathBuf>,
    ) -> Result<Self, SourceConfigError> {
        let (windows_root_override, wsl_root_override) = match root_override {
            Some(path) if is_wsl_path(&path) => (None, Some(path)),
            path => (path, None),
        };
        Self::try_new_with_roots(provider, enabled, windows_root_override, wsl_root_override)
    }

    pub fn try_new_with_roots(
        provider: Provider,
        enabled: bool,
        windows_root_override: Option<PathBuf>,
        wsl_root_override: Option<PathBuf>,
    ) -> Result<Self, SourceConfigError> {
        if let Some(path) = &windows_root_override {
            validate_windows_root(path)?;
        }
        if let Some(path) = &wsl_root_override {
            validate_wsl_root(path)?;
        }

        Ok(Self {
            provider,
            enabled,
            windows_root_override,
            wsl_root_override,
        })
    }

    pub fn defaults(provider: Provider) -> Self {
        Self {
            provider,
            enabled: true,
            windows_root_override: None,
            wsl_root_override: None,
        }
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn windows_root_override(&self) -> Option<&Path> {
        self.windows_root_override.as_deref()
    }

    pub fn wsl_root_override(&self) -> Option<&Path> {
        self.wsl_root_override.as_deref()
    }

    /// Compatibility accessor for callers that only support one source root.
    pub fn root_override(&self) -> Option<&Path> {
        self.windows_root_override()
            .or_else(|| self.wsl_root_override())
    }

    pub fn with_root_override(
        &self,
        platform: SourcePlatform,
        root_override: Option<PathBuf>,
    ) -> Result<Self, SourceConfigError> {
        let mut next = self.clone();
        match platform {
            SourcePlatform::Windows => {
                if let Some(path) = &root_override {
                    validate_windows_root(path)?;
                }
                next.windows_root_override = root_override;
            }
            SourcePlatform::Wsl => {
                if let Some(path) = &root_override {
                    validate_wsl_root(path)?;
                }
                next.wsl_root_override = root_override;
            }
        }
        Ok(next)
    }

    pub fn configured_root_label(&self) -> String {
        self.root_override()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| native_root_label(self.provider).to_owned())
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourcePlatform {
    Windows,
    Wsl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceConfigSet {
    configs: Vec<SourceConfig>,
}

impl SourceConfigSet {
    pub fn defaults() -> Self {
        Self {
            configs: Provider::all()
                .iter()
                .copied()
                .map(SourceConfig::defaults)
                .collect(),
        }
    }

    pub fn get(&self, provider: Provider) -> &SourceConfig {
        self.configs
            .iter()
            .find(|config| config.provider() == provider)
            .expect("every canonical provider should have source configuration")
    }

    pub fn replace(&mut self, config: SourceConfig) {
        if let Some(existing) = self
            .configs
            .iter_mut()
            .find(|existing| existing.provider() == config.provider())
        {
            *existing = config;
        } else {
            self.configs.push(config);
        }
    }

    pub fn is_enabled(&self, provider: Provider) -> bool {
        self.get(provider).enabled()
    }

    pub fn enabled_providers(&self) -> Vec<Provider> {
        self.configs
            .iter()
            .filter(|config| config.enabled())
            .map(SourceConfig::provider)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSourceConfigs {
    pub configs: SourceConfigSet,
    pub invalid_providers: Vec<Provider>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceConfigError {
    EmptyRoot,
    NulByte,
    RelativeRoot,
    UnsupportedUnc,
    DevicePath,
    ParentTraversal,
    InvalidRoot,
}

impl fmt::Display for SourceConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match self {
            Self::EmptyRoot => "empty_root",
            Self::NulByte => "nul_byte",
            Self::RelativeRoot => "relative_root",
            Self::UnsupportedUnc => "unsupported_unc",
            Self::DevicePath => "device_path",
            Self::ParentTraversal => "parent_traversal",
            Self::InvalidRoot => "invalid_root",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for SourceConfigError {}

pub fn parse_explicit_root(raw: &str) -> Result<PathBuf, SourceConfigError> {
    if raw.is_empty() {
        return Err(SourceConfigError::EmptyRoot);
    }
    if raw.contains('\0') {
        return Err(SourceConfigError::NulByte);
    }
    if raw.contains("://") {
        return Err(SourceConfigError::InvalidRoot);
    }

    let path = PathBuf::from(raw);
    validate_explicit_path(&path)?;
    Ok(path)
}

pub fn parse_windows_root(raw: &str) -> Result<PathBuf, SourceConfigError> {
    let path = parse_explicit_root(raw)?;
    if is_wsl_path(&path) {
        return Err(SourceConfigError::InvalidRoot);
    }
    Ok(path)
}

pub fn parse_wsl_root(raw: &str) -> Result<PathBuf, SourceConfigError> {
    let normalized = normalize_wsl_unc(raw);
    let path = parse_explicit_root(&normalized)?;
    if !is_wsl_path(&path) {
        return Err(SourceConfigError::InvalidRoot);
    }
    Ok(path)
}

fn normalize_wsl_unc(raw: &str) -> String {
    let separator_normalized = raw.replace('/', "\\");
    if !separator_normalized.starts_with(r"\\") {
        return raw.to_owned();
    }

    let components: Vec<_> = separator_normalized
        .split('\\')
        .filter(|component| !component.is_empty())
        .collect();
    if !components
        .first()
        .is_some_and(|server| server.eq_ignore_ascii_case("wsl.localhost"))
    {
        return raw.to_owned();
    }

    format!(r"\\{}", components.join("\\"))
}

pub(crate) fn is_wsl_path(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('/', "\\");
    let mut components = normalized
        .split('\\')
        .filter(|component| !component.is_empty());
    matches!(
        (components.next(), components.next()),
        (Some(server), Some(share))
            if server.eq_ignore_ascii_case("wsl.localhost") && !share.is_empty()
    )
}

fn validate_windows_root(path: &Path) -> Result<(), SourceConfigError> {
    validate_explicit_path(path)?;
    if is_wsl_path(path) {
        return Err(SourceConfigError::InvalidRoot);
    }
    Ok(())
}

fn validate_wsl_root(path: &Path) -> Result<(), SourceConfigError> {
    validate_explicit_path(path)?;
    if !is_wsl_path(path) {
        return Err(SourceConfigError::InvalidRoot);
    }
    Ok(())
}

fn validate_explicit_path(path: &Path) -> Result<(), SourceConfigError> {
    let mut saw_parent = false;
    let mut saw_current = false;
    for component in path.components() {
        match component {
            Component::ParentDir => saw_parent = true,
            Component::CurDir => saw_current = true,
            _ => {}
        }
    }
    if saw_parent {
        return Err(SourceConfigError::ParentTraversal);
    }
    if saw_current {
        return Err(SourceConfigError::InvalidRoot);
    }

    #[cfg(windows)]
    {
        use std::path::Prefix;

        if !path.is_absolute() {
            return Err(SourceConfigError::RelativeRoot);
        }
        let prefix = path.components().find_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.kind()),
            _ => None,
        });
        match prefix {
            Some(Prefix::Disk(_)) => {}
            Some(Prefix::UNC(server, share))
                if server
                    .to_string_lossy()
                    .eq_ignore_ascii_case("wsl.localhost")
                    && !share.is_empty() => {}
            Some(Prefix::UNC(_, _)) => return Err(SourceConfigError::UnsupportedUnc),
            Some(Prefix::Verbatim(_))
            | Some(Prefix::VerbatimUNC(_, _))
            | Some(Prefix::VerbatimDisk(_))
            | Some(Prefix::DeviceNS(_)) => return Err(SourceConfigError::DevicePath),
            None => return Err(SourceConfigError::InvalidRoot),
        }
    }

    #[cfg(not(windows))]
    if !path.is_absolute() {
        return Err(SourceConfigError::RelativeRoot);
    }

    Ok(())
}

fn native_root_label(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => ".claude",
        Provider::Codex => ".codex",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{parse_explicit_root, SourceConfig, SourceConfigSet, SourcePlatform};
    use crate::types::provider::Provider;

    #[test]
    fn defaults_enable_both_providers_and_use_native_labels() {
        let configs = SourceConfigSet::defaults();

        assert!(configs.is_enabled(Provider::Claude));
        assert!(configs.is_enabled(Provider::Codex));
        assert_eq!(
            configs.get(Provider::Claude).configured_root_label(),
            ".claude"
        );
        assert_eq!(
            configs.get(Provider::Codex).configured_root_label(),
            ".codex"
        );
    }

    #[cfg(windows)]
    #[test]
    fn dual_root_config_keeps_windows_and_wsl_selections_independent() {
        let config = SourceConfig::try_new_with_roots(
            Provider::Claude,
            true,
            Some(PathBuf::from(r"C:\Users\tester\.claude\projects")),
            Some(PathBuf::from(
                r"\\wsl.localhost\Ubuntu\home\tester\.claude\projects",
            )),
        )
        .unwrap();

        assert_eq!(
            config.windows_root_override(),
            Some(std::path::Path::new(r"C:\Users\tester\.claude\projects"))
        );
        assert_eq!(
            config.wsl_root_override(),
            Some(std::path::Path::new(
                r"\\wsl.localhost\Ubuntu\home\tester\.claude\projects",
            ))
        );

        let without_wsl = config
            .with_root_override(SourcePlatform::Wsl, None)
            .unwrap();
        assert!(without_wsl.windows_root_override().is_some());
        assert!(without_wsl.wsl_root_override().is_none());
    }

    #[cfg(windows)]
    #[test]
    fn explicit_root_accepts_approved_wsl_unc_shape() {
        let path =
            parse_explicit_root(r"\\wsl.localhost\Ubuntu\home\user\.claude\projects").unwrap();

        assert_eq!(
            path.to_string_lossy(),
            r"\\wsl.localhost\Ubuntu\home\user\.claude\projects"
        );
    }

    #[cfg(windows)]
    #[test]
    fn wsl_root_normalizes_redundant_unc_separators() {
        let path =
            super::parse_wsl_root(r"\\\wsl.localhost\Ubuntu\home\caocu\\.codex\sessions").unwrap();

        assert_eq!(
            path.to_string_lossy(),
            r"\\wsl.localhost\Ubuntu\home\caocu\.codex\sessions"
        );
    }

    #[cfg(windows)]
    #[test]
    fn explicit_root_rejects_arbitrary_unc_relative_and_device_paths() {
        assert!(parse_explicit_root(r"\\server\share\sessions").is_err());
        assert!(parse_explicit_root(r".claude\projects").is_err());
        assert!(parse_explicit_root(r"\\?\C:\sessions").is_err());
        assert!(parse_explicit_root(r"C:\sessions\..\outside").is_err());
    }
}
