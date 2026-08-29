//! Checkpoint queries and transaction support.

use crate::types::file_checkpoint::FileCheckpoint;

pub(crate) fn validate(checkpoint: &FileCheckpoint) -> Result<(), ()> {
    if checkpoint.file_identity.trim().is_empty()
        || checkpoint.byte_offset > checkpoint.size_bytes
        || checkpoint
            .pending_offset
            .is_some_and(|offset| offset > checkpoint.size_bytes)
    {
        return Err(());
    }
    Ok(())
}
