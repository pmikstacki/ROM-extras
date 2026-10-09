//! One-at-a-time durable page orchestration; no native transaction crosses remote I/O.
use crate::{
    ApprovedDocument, Cancellation, Error, PageIntent, PendingHistory, ProjectionHistory,
    ProjectionTarget, StorageWorker, TransactionId, WorkerFailure, WorkerResult, codec, limits,
};
use rom::JournalCursor;
use std::collections::BTreeMap;
/// Exclusive page coordinator over a trusted target, with one mutable operation at a time.
/// Construction/error/drop and shutdown still require a blocking owner-lifecycle host context.
/// Qualified targets must fence requests surviving cancellation; this type cannot supply that fence.
pub struct Worker<T: ProjectionTarget> {
    storage: StorageWorker,
    target: T,
}
impl<T: ProjectionTarget> Worker<T> {
    /// Bind the actual native profile to a fixed approved target; does not qualify its transport.
    pub fn new(storage: StorageWorker, target: T) -> WorkerResult<Self> {
        if storage.profile() != target.profile() {
            return Err(Error::Conflict.into());
        }
        limits::identifier(target.physical_target(), limits::IDENTIFIER)?;
        Ok(Self { storage, target })
    }
    fn binding(&self, checkpoint: &crate::Checkpoint) -> WorkerResult<()> {
        if self.target.profile() != self.storage.profile()
            || self.target.physical_target() != checkpoint.physical_target()
        {
            return Err(Error::Conflict.into());
        }
        Ok(())
    }
    fn request(
        &self,
        page: &PageIntent,
        documents: Vec<ApprovedDocument>,
    ) -> WorkerResult<Option<T::Request>> {
        self.binding(&page.expected)?;
        if documents.len() != page.operations().len() {
            return Err(Error::Conflict.into());
        }
        for (doc, op) in documents.iter().zip(page.operations()) {
            if doc.profile() != self.storage.profile() || doc.metadata() != op {
                return Err(Error::Conflict.into());
            }
        }
        if documents.is_empty() {
            return Ok(None);
        }
        let references: Vec<_> = documents.iter().collect();
        // Preparation owns its bounded request. Approved values can drop before native preparation.
        Ok(Some(self.target.prepare(&references)?))
    }
    /// Validate the complete selected request, prepare durably, then dispatch and observe exactly.
    /// Unknown/future cancellation retains pending work; call recover before a later page.
    pub async fn apply_page(
        &mut self,
        next: JournalCursor,
        documents: Vec<ApprovedDocument>,
        cancel: &Cancellation,
    ) -> WorkerResult<TransactionId> {
        cancel.check()?;
        if documents.len() > limits::OPERATIONS {
            return Err(Error::TooLarge.into());
        }
        if documents
            .iter()
            .any(|d| d.profile() != self.storage.profile())
        {
            return Err(Error::Conflict.into());
        }
        let snapshot = self.snapshot().await?;
        if snapshot.pending_page().is_some() {
            return Err(WorkerFailure::RecoveryRequired);
        }
        let page = PageIntent::new(
            snapshot.checkpoint(),
            next,
            documents.iter().map(|d| d.metadata().clone()).collect(),
        )?;
        // Admission must satisfy the same interval bound as durable recovery.
        let _history = PendingHistory::new(page.clone())?;
        let request = self.request(&page, selected(documents))?;
        cancel.check()?;
        let _prepared = self.storage.prepare_page(page.clone())?.receive().await?;
        self.finish(page, request, cancel).await
    }
    /// Reconstruct complete old authorized history before replay or publication.
    /// No pending page returns None. Source qualification and immutable grant policy are host duties.
    pub async fn recover(
        &mut self,
        source: &mut impl ProjectionHistory,
        cancel: &Cancellation,
    ) -> WorkerResult<Option<TransactionId>> {
        cancel.check()?;
        if source.profile() != self.storage.profile() {
            return Err(Error::RebuildRequired.into());
        }
        let snapshot = self.snapshot().await?;
        self.binding(snapshot.checkpoint())?;
        let Some(page) = snapshot.pending_page().cloned() else {
            return Ok(None);
        };
        let mut history = PendingHistory::new(page.clone())?;
        let mut documents = BTreeMap::new();
        while history.cursor() != page.next_cursor() {
            cancel.check()?;
            history.check_fetch_budget()?;
            let batch = source.fetch(history.cursor()).await?;
            if source.profile() != self.storage.profile() {
                return Err(Error::RebuildRequired.into());
            }
            history.push(&batch, cancel, |event| {
                let document = source.document(event)?;
                if document.profile() != self.storage.profile() {
                    return Err(Error::RebuildRequired);
                }
                let operation = document.metadata().clone();
                documents.insert(codec::key(operation.key()), document);
                if documents.len() > limits::OPERATIONS {
                    return Err(Error::TooLarge);
                }
                Ok(operation)
            })?;
        }
        if source.profile() != self.storage.profile() {
            return Err(Error::RebuildRequired.into());
        }
        let page = history.finish()?;
        let request = self.request(&page, documents.into_values().collect())?;
        Ok(Some(self.finish(page, request, cancel).await?))
    }
    async fn finish(
        &mut self,
        page: PageIntent,
        request: Option<T::Request>,
        cancel: &Cancellation,
    ) -> WorkerResult<TransactionId> {
        cancel.check()?;
        let observations = match request {
            Some(request) => self.target.apply(request).await?,
            None => Vec::new(),
        };
        cancel.check()?;
        let observed = page.reconcile(self.storage.profile(), observations)?;
        // Once admitted, local completion must classify its actual result despite late cancellation.
        Ok(self
            .storage
            .complete_page(page, observed)?
            .receive()
            .await?)
    }
    /// Read durable state without network I/O; never infer rollback from an absent caller response.
    pub async fn snapshot(&self) -> WorkerResult<crate::StoreSnapshot> {
        Ok(self.storage.load()?.receive().await?)
    }
    /// Explicitly reopen a retired native engine while retaining ownership and uncertainty.
    pub async fn reopen(&self, cancel: Cancellation) -> WorkerResult<()> {
        Ok(self.storage.reopen(cancel)?.receive().await?)
    }
    /// Classify an exact retained native token after reopening; no automatic retry is performed.
    pub async fn reconcile(&self, token: TransactionId) -> WorkerResult<crate::CommitStatus> {
        Ok(self.storage.reconcile(token)?.receive().await?)
    }
    /// Close admission, drain and join native destruction from a bounded blocking host context.
    pub fn shutdown(&self) -> WorkerResult<()> {
        Ok(self.storage.shutdown()?)
    }
}
fn selected(documents: Vec<ApprovedDocument>) -> Vec<ApprovedDocument> {
    let mut selected = BTreeMap::new();
    for document in documents {
        selected.insert(codec::key(document.metadata().key()), document);
    }
    selected.into_values().collect()
}
