//! Typed bounded checkpoint commands on a dedicated native owner thread.
use crate::{
    Cancellation, Checkpoint, CheckpointStore, Error, PageIntent, ProjectionProfile,
    ReconciledPage, StoreSnapshot, TransactionId,
};
use std::path::PathBuf;
/// Storage admission or uncertain response failure. Does not infer rollback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageFailure {
    /// A classified checkpoint result, including its exact uncertain transaction token.
    Checkpoint(Error),
    /// Command queue was full; this command was not admitted.
    Overloaded,
    /// Admission closed; this command was not admitted.
    Closed,
    /// An admitted response or initialization could not be classified.
    Unclassified,
}
impl std::fmt::Display for StorageFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("projection storage worker failure")
    }
}
impl std::error::Error for StorageFailure {}
/// One admitted command. Dropping a response does not cancel durable work.
pub struct StorageResponse<T> {
    receiver: tokio::sync::oneshot::Receiver<crate::Result<T>>,
}
impl<T> StorageResponse<T> {
    /// Await a response without blocking the asynchronous caller.
    pub async fn receive(self) -> std::result::Result<T, StorageFailure> {
        self.receiver
            .await
            .map_err(|_| StorageFailure::Unclassified)?
            .map_err(StorageFailure::Checkpoint)
    }
}
/// Non-cloneable owner of one thread and one queued command, excluding active work.
/// Initialization, shutdown and drop block. Use a blocking host context for these operations.
/// Awaiting command responses does not block. Dropping a response never cancels admitted work.
pub struct StorageWorker {
    executor: rom_sql_core::Executor<CheckpointStore>,
}
impl StorageWorker {
    /// Create native storage on its owner thread. Initialization can block on local I/O.
    pub fn create(
        path: PathBuf,
        profile: ProjectionProfile,
        initial: Checkpoint,
    ) -> std::result::Result<Self, StorageFailure> {
        Self::spawn(move || CheckpointStore::create(&path, &profile, &initial))
    }
    /// Open native storage on its owner thread with cooperative startup cancellation.
    pub fn open(
        path: PathBuf,
        profile: ProjectionProfile,
        cancel: Cancellation,
    ) -> std::result::Result<Self, StorageFailure> {
        Self::spawn(move || CheckpointStore::open_cancellable(&path, &profile, &cancel))
    }
    fn spawn(
        initialize: impl FnOnce() -> crate::Result<CheckpointStore> + Send + 'static,
    ) -> std::result::Result<Self, StorageFailure> {
        let (sender, response) = std::sync::mpsc::sync_channel(1);
        match rom_sql_core::Executor::spawn(1, move || {
            initialize().map_err(|error| {
                let _ = sender.send(error);
                rom_sql_core::ExecutorError::Initialization
            })
        }) {
            Ok(executor) => Ok(Self { executor }),
            Err(_) => Err(response
                .try_recv()
                .map(StorageFailure::Checkpoint)
                .unwrap_or(StorageFailure::Unclassified)),
        }
    }
    // Only this module constructs closures; every public capture has validated bounded metadata.
    fn submit<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut CheckpointStore) -> crate::Result<T> + Send + 'static,
    ) -> std::result::Result<StorageResponse<T>, StorageFailure> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let ticket = self
            .executor
            .submit(move |store| {
                let _ = sender.send(operation(store));
            })
            .map_err(admission_error)?;
        drop(ticket); // The command owns its independent one-shot result until native completion.
        Ok(StorageResponse { receiver })
    }
    /// Submit a bounded state read without waiting for queue space.
    pub fn load(&self) -> std::result::Result<StorageResponse<StoreSnapshot>, StorageFailure> {
        self.submit(|store| store.load())
    }
    /// Submit immutable intent before remote dispatch; admission is not commit acknowledgement.
    pub fn prepare_page(
        &self,
        page: PageIntent,
    ) -> std::result::Result<StorageResponse<TransactionId>, StorageFailure> {
        self.submit(move |store| store.prepare_page(&page))
    }
    /// Submit exact reconciliation and atomic key/cursor publication.
    pub fn complete_page(
        &self,
        page: PageIntent,
        observed: ReconciledPage,
    ) -> std::result::Result<StorageResponse<TransactionId>, StorageFailure> {
        self.submit(move |store| store.complete_page(&page, &observed))
    }
    /// Read one original key after validating its capture bounds.
    pub fn key_state(
        &self,
        key: rom::Key,
    ) -> std::result::Result<StorageResponse<Option<crate::KeyState>>, StorageFailure> {
        crate::limits::key(&key).map_err(StorageFailure::Checkpoint)?;
        self.submit(move |store| store.key_state(&key))
    }
    /// Classify the exact transaction after reopening a retired engine.
    pub fn reconcile(
        &self,
        transaction: TransactionId,
    ) -> std::result::Result<StorageResponse<crate::CommitStatus>, StorageFailure> {
        self.submit(move |store| store.reconcile(&transaction))
    }
    /// Reopen while retaining this owner's reservation and uncertainty metadata.
    pub fn reopen(
        &self,
        cancel: Cancellation,
    ) -> std::result::Result<StorageResponse<()>, StorageFailure> {
        self.submit(move |store| store.reopen_cancellable(&cancel))
    }
    /// Stop admission, drain commands, and join through engine destruction.
    /// This blocks on admitted native I/O and has no hard shutdown deadline.
    pub fn shutdown(&self) -> std::result::Result<(), StorageFailure> {
        self.executor.shutdown().map_err(admission_error)
    }
}
impl Drop for StorageWorker {
    fn drop(&mut self) {
        let _ = self.executor.shutdown();
    }
}
fn admission_error(error: rom_sql_core::ExecutorError) -> StorageFailure {
    match error {
        rom_sql_core::ExecutorError::Overloaded => StorageFailure::Overloaded,
        rom_sql_core::ExecutorError::Closed => StorageFailure::Closed,
        _ => StorageFailure::Unclassified,
    }
}

#[cfg(test)]
#[path = "storage_worker_tests.rs"]
mod tests;
