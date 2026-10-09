//! Durable bounded host projection checkpoints. Provider integration is not yet qualified.
mod cancellation;
mod checkpoint;
mod codec;
mod error;
mod limits;
mod metadata;
mod preflight;
pub use cancellation::Cancellation;
pub use checkpoint::CheckpointStore;
pub use error::{Error, Result};
pub use metadata::{
    Checkpoint, CommitStatus, KeyState, OperationMetadata, PageIntent, ProjectionProfile,
    ReconciledPage, RemoteObservation, StoreSnapshot, TransactionId,
};

#[cfg(test)]
extern crate self as rom_projection_core;

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

mod storage_worker;
pub use storage_worker::{StorageFailure, StorageResponse, StorageWorker};

mod history;
pub use history::PendingHistory;
