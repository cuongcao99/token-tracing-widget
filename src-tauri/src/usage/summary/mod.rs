//! Normalized rows used by the Usage Summary calculation.

use crate::types::rate_limit::ProviderRateLimitSummary;
use crate::types::usage_event::UsageEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRows {
    pub events: Vec<UsageEvent>,
    pub rate_limits: Vec<ProviderRateLimitSummary>,
}
