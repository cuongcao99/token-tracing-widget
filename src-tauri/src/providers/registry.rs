//! Canonical built-in Provider and adapter registrations.

use super::claude::ClaudeReader;
use super::codex::CodexReader;
use super::provider_adapter::ProviderAdapter;
use crate::types::provider::Provider;

static CLAUDE_READER: ClaudeReader = ClaudeReader;
static CODEX_READER: CodexReader = CodexReader;

static REGISTRATIONS: [ProviderRegistration; 2] = [
    ProviderRegistration {
        provider: Provider::Claude,
        adapter: &CLAUDE_READER,
    },
    ProviderRegistration {
        provider: Provider::Codex,
        adapter: &CODEX_READER,
    },
];

#[derive(Clone, Copy)]
pub struct ProviderRegistration {
    provider: Provider,
    adapter: &'static dyn ProviderAdapter,
}

impl ProviderRegistration {
    pub const fn provider(self) -> Provider {
        self.provider
    }

    pub const fn adapter(self) -> &'static dyn ProviderAdapter {
        self.adapter
    }
}

#[derive(Clone, Copy)]
pub struct ProviderRegistry {
    registrations: &'static [ProviderRegistration],
}

impl ProviderRegistry {
    pub fn registrations(&self) -> impl Iterator<Item = ProviderRegistration> + '_ {
        self.registrations.iter().copied()
    }

    pub fn providers(&self) -> impl Iterator<Item = Provider> + '_ {
        self.registrations().map(ProviderRegistration::provider)
    }

    pub fn registration(&self, provider: Provider) -> Option<ProviderRegistration> {
        self.registrations
            .iter()
            .copied()
            .find(|registration| registration.provider == provider)
    }
}

pub const fn provider_registry() -> ProviderRegistry {
    ProviderRegistry {
        registrations: &REGISTRATIONS,
    }
}
