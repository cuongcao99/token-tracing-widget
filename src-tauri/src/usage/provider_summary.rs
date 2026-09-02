//! Deriving one privacy-safe display summary for a supported Provider.

use crate::types::provider::Provider;
use crate::types::provider_usage_summary::ProviderUsageSummary;
use crate::types::rate_limit::RateLimitSummary;
use crate::types::session_usage_summary::{
    safe_session_id, SessionUsageState, SessionUsageSummary,
};
use crate::types::source_health::SourceHealth;
use crate::types::usage_event::UsageEvent;
use crate::usage::active_provider::{
    compute_active_provider, compute_current_session_tokens_for_local_day,
};
use crate::usage::daily_total::compute_today_total;
use crate::UsageState;

pub fn compute_provider_summary(
    provider: Provider,
    events: &[UsageEvent],
    health: Option<&SourceHealth>,
    rate_limits: &[RateLimitSummary],
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

    let sessions = crate::usage::session_summary::compute_session_aggregation(
        &provider_events,
        now,
        Some(local_day),
    )
    .sessions
    .into_iter()
    .map(|session| SessionUsageSummary {
        id: safe_session_id(&session.session_key),
        name: session.name,
        state: if session.active {
            SessionUsageState::Active
        } else {
            SessionUsageState::Idle
        },
        today_tokens: session.current_day_tokens,
    })
    .collect();

    let mut summary = ProviderUsageSummary::new(
        provider,
        state,
        compute_current_session_tokens_for_local_day(&provider_events, now, local_day),
        compute_today_total(&provider_events, local_day),
        active.last_updated_at,
        sessions,
    );
    summary.rate_limits = rate_limits.to_vec();
    summary
}

fn source_is_usable(health: Option<&SourceHealth>) -> bool {
    health
        .is_some_and(|health| matches!(health.state.as_str(), "detected" | "limited" | "malformed"))
}
