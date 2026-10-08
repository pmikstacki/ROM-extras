//! Durable bounded host projection checkpoints. Provider integration is not yet qualified.
mod checkpoint;
mod codec;
mod error;
mod limits;
mod metadata;
mod preflight;
pub use checkpoint::CheckpointStore;
pub use error::{Error, Result};
pub use metadata::{
    Checkpoint, CommitStatus, KeyState, OperationMetadata, PageIntent, ProjectionProfile,
    ReconciledPage, RemoteObservation, StoreSnapshot, TransactionId,
};
