//! Persisted provider source selection and explicit-root input validation.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::types::provider::Provider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceConfig {
    provider: Provider,
    enabled: bool,
    root_override: Option<PathBuf>,
}

impl SourceConfig {
    pub fn try_new(
        provider: Provider,
        enabled: bool,
        root_override: Option<PathBuf>,
    ) -> Result<Self, SourceConfigError> {
        if let Some(path) = &root_override {
            validate_explicit_path(path)?;
        }

        Ok(Self {
            provider,
            enabled,
            root_override,
        })
    }

    pub fn defaults(provider: Provider) -> Self {
        Self {
            provider,
            enabled: true,
            root_override: None,
        }
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn root_override(&self) -> Option<&Path> {
        self.root_override.as_deref()
    }

    pub fn configured_root_label(&self) -> String {
        self.root_override
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| native_root_label(self.provider).to_owned())
    }
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
        Provider::Claude => ".claude/projects",
        Provider::Codex => ".codex/sessions",
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_explicit_root, SourceConfigSet};
    use crate::types::provider::Provider;

    #[test]
    fn defaults_enable_both_providers_and_use_native_labels() {
        let configs = SourceConfigSet::defaults();

        assert!(configs.is_enabled(Provider::Claude));
        assert!(configs.is_enabled(Provider::Codex));
        assert_eq!(
            configs.get(Provider::Claude).configured_root_label(),
            ".claude/projects"
        );
        assert_eq!(
            configs.get(Provider::Codex).configured_root_label(),
            ".codex/sessions"
        );
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
    fn explicit_root_rejects_arbitrary_unc_relative_and_device_paths() {
        assert!(parse_explicit_root(r"\\server\share\sessions").is_err());
        assert!(parse_explicit_root(r".claude\projects").is_err());
        assert!(parse_explicit_root(r"\\?\C:\sessions").is_err());
        assert!(parse_explicit_root(r"C:\sessions\..\outside").is_err());
    }
}
