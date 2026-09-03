//! Collection inputs shared by the pure core and the persistence Adapter.

mod coordinator;
mod persistence;
mod source_collection;

pub use crate::usage::summary::SummaryRows;
pub use coordinator::{
    compute_summary, CollectionClock, CollectionCoordinator, CollectionError, CollectionReport,
    FixedClock, ProviderSource, WindowsClock,
};
pub use persistence::{
    CollectionBatch, CollectionStore, CollectionStoreError, DiagnosticUpdate, RateLimitUpdate,
    SessionKeyUpdate, SessionNameUpdate, SourceUpdate,
};
