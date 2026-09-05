use std::collections::BTreeSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::app::runtime::{AppState, RuntimeError};
use crate::collection::CollectionReport;
use crate::commands::usage_summary::SummaryEventError;
use crate::sources::file_watcher::{SourceObserver, WatchSignal};
use crate::sources::source_config::SourceConfig;
use crate::types::provider::Provider;
use crate::types::usage_summary::UsageSummary;
use crate::usage::summary::ACTIVE_SESSION_WINDOW_SECONDS;
use crate::UsageState;

use super::scheduler::{CollectionReason, LiveCollectionConfig, LiveScheduler};

pub(super) trait CollectionBackend: Send {
    fn collect(&mut self) -> Result<CollectionReport, RuntimeError>;
}

pub(super) trait SummaryPublisher: Send {
    fn publish(&mut self, summary: &UsageSummary) -> Result<(), SummaryEventError>;
}

pub(super) struct LiveCollectionController<B, P> {
    state: AppState,
    backend: B,
    publisher: P,
    observer: SourceObserver,
    scheduler: LiveScheduler,
    observed_providers: BTreeSet<Provider>,
}

impl<B, P> LiveCollectionController<B, P>
where
    B: CollectionBackend,
    P: SummaryPublisher,
{
    pub(super) fn new(
        state: AppState,
        backend: B,
        publisher: P,
        observer: SourceObserver,
        start: Instant,
        config: LiveCollectionConfig,
    ) -> Self {
        Self {
            state,
            backend,
            publisher,
            observer,
            scheduler: LiveScheduler::new(start, config),
            observed_providers: BTreeSet::new(),
        }
    }

    pub(super) fn on_signal(&mut self, signal: WatchSignal, now: Instant) -> bool {
        match signal {
            WatchSignal::Changed(provider) | WatchSignal::WatchUnavailable(provider)
                if self.observed_providers.contains(&provider) =>
            {
                self.scheduler.mark_changed(now);
                true
            }
            WatchSignal::Changed(_) | WatchSignal::WatchUnavailable(_) => true,
            WatchSignal::ConfigurationChanged => {
                self.refresh_observers(now);
                true
            }
            WatchSignal::Shutdown => false,
        }
    }

    pub(super) fn process_due(&mut self, now: Instant) -> Option<CollectionReason> {
        let reason = self.scheduler.take_due(now)?;
        if reason == CollectionReason::Reconciliation {
            self.refresh_observers(now);
        }
        match self.backend.collect() {
            Ok(report) => {
                let has_pending_reads = report.has_pending_reads;
                if report.summary.state == UsageState::Active {
                    if report.accepted_event_count > 0 || !self.scheduler.has_activity_expiry() {
                        self.scheduler.arm_activity_expiry(
                            now + Duration::from_secs(ACTIVE_SESSION_WINDOW_SECONDS as u64),
                        );
                    }
                } else {
                    self.scheduler.clear_activity_expiry();
                }
                let publish_error = self.publisher.publish(&report.summary).err();
                self.scheduler.record_success();
                if has_pending_reads && !self.observed_providers.is_empty() {
                    self.scheduler.mark_changed(now);
                }
                if publish_error.is_some() {
                    eprintln!("summary_event:emit");
                }
            }
            Err(_) => {
                let mut summary = self.state.summary();
                if summary.state == UsageState::Loading {
                    summary = UsageSummary::unavailable();
                }
                if self.publisher.publish(&summary).is_err() {
                    eprintln!("summary_event:emit");
                }
                self.scheduler.record_failure(now);
            }
        }
        Some(reason)
    }

    pub(super) fn refresh_observers(&mut self, now: Instant) {
        let roots = self.state.watch_roots();
        for provider in self.observed_providers.clone() {
            self.observer.stop_provider(provider);
        }
        self.observed_providers.clear();
        if self.observed_providers.is_empty() {
            self.scheduler.deactivate();
        }

        for root in roots {
            let provider = root.provider();
            self.observer.start_provider(root);
            if self.observed_providers.insert(provider) {
                self.scheduler.activate(now);
                self.scheduler.mark_changed(now);
            }
        }

        if !self.observed_providers.is_empty() {
            self.scheduler.mark_changed(now);
        }
    }

    pub(super) fn run(mut self, receiver: Receiver<WatchSignal>) {
        let start = Instant::now();
        self.refresh_observers(start);
        // Always schedule an initial summary, including when no source root exists yet.
        self.scheduler.mark_changed(start);
        loop {
            if self.process_due(Instant::now()).is_some() {
                continue;
            }

            let signal = match self.scheduler.next_deadline() {
                Some(deadline) => match receiver
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                {
                    Ok(signal) => signal,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                },
                None => match receiver.recv() {
                    Ok(signal) => signal,
                    Err(_) => break,
                },
            };
            if !self.on_signal(signal, Instant::now()) {
                break;
            }
        }

        self.observer.shutdown();
    }
}

pub(crate) struct LiveCollectionHandle {
    sender: std::sync::Mutex<Option<std::sync::mpsc::Sender<WatchSignal>>>,
    worker: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl LiveCollectionHandle {
    pub(crate) fn request_source_refresh(&self) -> bool {
        let Ok(sender) = self.sender.lock() else {
            return false;
        };
        sender
            .as_ref()
            .is_some_and(|sender| sender.send(WatchSignal::ConfigurationChanged).is_ok())
    }

    pub(crate) fn shutdown(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            if let Some(sender) = sender.take() {
                let _ = sender.send(WatchSignal::Shutdown);
            }
        }
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }

    #[cfg(test)]
    pub(super) fn from_parts(
        sender: std::sync::mpsc::Sender<WatchSignal>,
        worker: JoinHandle<()>,
    ) -> Self {
        Self {
            sender: std::sync::Mutex::new(Some(sender)),
            worker: std::sync::Mutex::new(Some(worker)),
        }
    }
}

pub(crate) fn update_source_config_and_refresh(
    state: &AppState,
    handle: &LiveCollectionHandle,
    config: SourceConfig,
) -> Result<(), RuntimeError> {
    state.update_source_config(config)?;
    let _ = handle.request_source_refresh();
    Ok(())
}

impl Drop for LiveCollectionHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;

pub(crate) fn start_live_collection(
    state: AppState,
    app: tauri::AppHandle,
) -> LiveCollectionHandle {
    let (sender, receiver) = std::sync::mpsc::channel();
    let observer_sender = sender.clone();
    let collection_state = state.clone();
    let collection_app = app.clone();
    let worker = thread::Builder::new()
        .name("live-collection".to_owned())
        .spawn(move || {
            let observer = SourceObserver::new(observer_sender);
            LiveCollectionController::new(
                state,
                super::adapters::RuntimeBackend::new(collection_state),
                super::adapters::TauriSummaryPublisher::new(collection_app),
                observer,
                Instant::now(),
                LiveCollectionConfig::default(),
            )
            .run(receiver);
        })
        .expect("live collection worker should start");

    LiveCollectionHandle {
        sender: std::sync::Mutex::new(Some(sender)),
        worker: std::sync::Mutex::new(Some(worker)),
    }
}
