//! Normalized rows used by the Usage Summary calculation.

mod active_provider;
mod daily_total;
mod provider;
mod sessions;

use crate::types::provider::Provider;
use crate::types::rate_limit::ProviderRateLimitSummary;
use crate::types::source_health::SourceHealth;
use crate::types::usage_event::UsageEvent;
use crate::types::usage_summary::UsageSummary;
use crate::UsageState;

use active_provider::{compute_active_provider, compute_current_session_tokens_for_local_day};
use daily_total::compute_today_total;
use provider::compute_provider_summary;

pub use sessions::ACTIVE_SESSION_WINDOW_SECONDS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRows {
    pub events: Vec<UsageEvent>,
    pub rate_limits: Vec<ProviderRateLimitSummary>,
}

pub fn compute_summary(
    rows: &SummaryRows,
    source_health: &[SourceHealth],
    enabled_providers: &[Provider],
    now: &str,
    local_day: &str,
) -> UsageSummary {
    let enabled_events: Vec<_> = rows
        .events
        .iter()
        .filter(|event| enabled_providers.contains(&event.provider))
        .cloned()
        .collect();
    let active = compute_active_provider(&enabled_events, now);
    let usable_source = source_health
        .iter()
        .any(|health| matches!(health.state.as_str(), "detected" | "limited" | "malformed"));
    let state = if active.state == UsageState::Active {
        UsageState::Active
    } else if usable_source {
        UsageState::Idle
    } else {
        UsageState::Unavailable
    };
    let providers = Provider::all()
        .iter()
        .copied()
        .map(|provider| {
            let health = source_health
                .iter()
                .find(|entry| entry.provider == provider);
            let rate_limits: Vec<_> = if enabled_providers.contains(&provider)
                && health.is_some_and(|health| {
                    matches!(health.state.as_str(), "detected" | "limited" | "malformed")
                }) {
                rows.rate_limits
                    .iter()
                    .filter(|entry| entry.provider == provider)
                    .map(|entry| entry.rate_limit)
                    .collect()
            } else {
                Vec::new()
            };
            compute_provider_summary(
                provider,
                &enabled_events,
                health,
                &rate_limits,
                now,
                local_day,
            )
        })
        .collect();

    UsageSummary {
        state,
        provider: active.provider,
        current_session_tokens: compute_current_session_tokens_for_local_day(
            &enabled_events,
            now,
            local_day,
        ),
        today_tokens: compute_today_total(&enabled_events, local_day),
        last_updated_at: active.last_updated_at,
        source_health: source_health.to_vec(),
        providers,
    }
}
