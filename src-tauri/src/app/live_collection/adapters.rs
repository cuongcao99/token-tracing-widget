use crate::app::runtime::{AppState, RuntimeError};
use crate::collection::{CollectionReport, WindowsClock};
use crate::commands::usage_summary::{emit_usage_summary, SummaryEventError};
use crate::types::usage_summary::UsageSummary;

use super::controller::{CollectionBackend, SummaryPublisher};

pub(super) struct RuntimeBackend {
    state: AppState,
}

impl RuntimeBackend {
    pub(super) fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl CollectionBackend for RuntimeBackend {
    fn collect(&mut self) -> Result<CollectionReport, RuntimeError> {
        self.state.collect_once(&WindowsClock::current())
    }
}

pub(super) struct TauriSummaryPublisher {
    app: tauri::AppHandle,
}

impl TauriSummaryPublisher {
    pub(super) fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl SummaryPublisher for TauriSummaryPublisher {
    fn publish(&mut self, summary: &UsageSummary) -> Result<(), SummaryEventError> {
        emit_usage_summary(&self.app, summary)
    }
}
