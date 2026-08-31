//! Deriving one privacy-safe display summary for a supported Provider.

use crate::types::provider::Provider;
use crate::types::provider_usage_summary::ProviderUsageSummary;
use crate::types::source_health::SourceHealth;
use crate::types::usage_event::UsageEvent;
use crate::usage::active_provider::compute_active_provider;
use crate::usage::daily_total::compute_today_total;
use crate::UsageState;

pub fn compute_provider_summary(
    provider: Provider,
    events: &[UsageEvent],
    health: Option<&SourceHealth>,
    now: &str,
    local_day: &str,
) -> ProviderUsageSummary {
    let provider_events: Vec<UsageEvent> = events
        .iter()
        .filter(|event| event.provider == provider)
        .cloned()
        .collect();
    let active = compute_active_provider(&provider_events, now);
    let state = if provider_events.is_empty() && !source_is_usable(health) {
        UsageState::Unavailable
    } else if active.state == UsageState::Active {
        UsageState::Active
    } else {
        UsageState::Idle
    };

    ProviderUsageSummary::new(
        provider,
        state,
        active.current_session_tokens,
        compute_today_total(&provider_events, local_day),
        active.last_updated_at,
    )
}

fn source_is_usable(health: Option<&SourceHealth>) -> bool {
    health
        .is_some_and(|health| matches!(health.state.as_str(), "detected" | "limited" | "malformed"))
}
