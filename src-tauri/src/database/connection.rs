//! Opening the local SQLite database.

use std::path::Path;

use rusqlite::Connection;

use crate::collection::CollectionBatch;
use crate::types::file_checkpoint::FileCheckpoint;
use crate::types::usage_event::UsageEvent;

use super::{diagnostics, file_checkpoints, schema, sessions, settings, sources, usage_events};
use crate::sources::source_config::{LoadedSourceConfigs, SourceConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    Open,
    Schema,
    Read,
    Write,
    InvalidValue,
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let category = match self {
            Self::Open => "open",
            Self::Schema => "schema",
            Self::Read => "read",
            Self::Write => "write",
            Self::InvalidValue => "invalid_value",
        };
        formatter.write_str(category)
    }
}

impl std::error::Error for StorageError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRows {
    pub events: Vec<UsageEvent>,
}

pub struct IndexStore {
    connection: Connection,
}

impl IndexStore {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let connection = Connection::open(path).map_err(|_| StorageError::Open)?;
        schema::initialize(&connection).map_err(|_| StorageError::Schema)?;
        Ok(Self { connection })
    }

    pub fn load_source_configs(&self) -> Result<LoadedSourceConfigs, StorageError> {
        settings::load_source_configs(&self.connection).map_err(|_| StorageError::Read)
    }

    pub fn save_source_config(&mut self, config: &SourceConfig) -> Result<(), StorageError> {
        if config
            .root_override()
            .is_some_and(|path| path.to_str().is_none())
        {
            return Err(StorageError::InvalidValue);
        }

        let transaction = self
            .connection
            .transaction()
            .map_err(|_| StorageError::Write)?;
        settings::save_source_config(&transaction, config).map_err(|_| StorageError::Write)?;
        transaction.commit().map_err(|_| StorageError::Write)
    }

    pub fn load_checkpoint(&self, identity: &str) -> Result<Option<FileCheckpoint>, StorageError> {
        if identity.trim().is_empty() {
            return Err(StorageError::InvalidValue);
        }
        file_checkpoints::load(&self.connection, identity).map_err(|_| StorageError::Read)
    }

    pub fn apply_batch(&mut self, batch: &CollectionBatch) -> Result<(), StorageError> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|_| StorageError::Write)?;

        for event in &batch.events {
            usage_events::insert(&transaction, event).map_err(|_| StorageError::Write)?;
            sessions::upsert(&transaction, event).map_err(|_| StorageError::Write)?;
        }
        for source in &batch.source_updates {
            sources::upsert(&transaction, source).map_err(|_| StorageError::Write)?;
        }
        for diagnostic in &batch.diagnostics {
            diagnostics::upsert(&transaction, diagnostic).map_err(|_| StorageError::Write)?;
        }
        for checkpoint in &batch.checkpoints {
            file_checkpoints::upsert(&transaction, checkpoint).map_err(|_| StorageError::Write)?;
        }

        transaction.commit().map_err(|_| StorageError::Write)
    }

    pub fn query_events_for_summary(
        &self,
        day_start: &str,
        now: &str,
    ) -> Result<SummaryRows, StorageError> {
        usage_events::query_between(&self.connection, day_start, now)
            .map(|events| SummaryRows { events })
            .map_err(|_| StorageError::Read)
    }

    pub fn count_usage_events(&self) -> Result<u64, StorageError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM usage_events", [], |row| row.get(0))
            .map_err(|_| StorageError::Read)?;
        u64::try_from(count).map_err(|_| StorageError::Read)
    }
}
