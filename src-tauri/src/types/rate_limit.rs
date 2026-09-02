//! Privacy-safe provider rate-limit metadata.

use serde::Serialize;

use super::provider::Provider;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitSummary {
    pub window_minutes: u32,
    pub used_percent: u8,
    pub resets_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitSnapshot {
    pub window_minutes: u32,
    pub used_percent: u8,
    pub resets_at: u64,
    pub observed_at: String,
}

impl RateLimitSnapshot {
    pub fn summary(&self) -> RateLimitSummary {
        RateLimitSummary {
            window_minutes: self.window_minutes,
            used_percent: self.used_percent,
            resets_at: self.resets_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRateLimitSummary {
    pub provider: Provider,
    pub rate_limit: RateLimitSummary,
}
