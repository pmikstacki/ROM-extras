//! Trusted host seams for bounded page preparation and actual stored-state observation.
use crate::{ApprovedDocument, Error, ProjectionProfile, RemoteObservation, StorageFailure};
use rom::{JournalBatch, JournalCursor, JournalView};
use std::future::Future;
/// Sanitized backend classification. No variant grants rollback or automatic retry permission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetFailure {
    /// Request outcome is not established.
    Unknown,
    /// Backend rejected the request; pending durable work still needs resolution.
    Rejected,
    /// Backend cannot currently establish its state.
    Unavailable,
}
/// Page orchestration classification, retaining exact native uncertainty tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerFailure {
    /// Core metadata, mapping or history failure.
    Core(Error),
    /// Native storage/admission failure, without losing its classified token.
    Storage(StorageFailure),
    /// Backend failure, with pending intent retained.
    Target(TargetFailure),
    /// Existing durable work must resolve before later page admission.
    RecoveryRequired,
}
/// Bounded worker result.
pub type WorkerResult<T> = std::result::Result<T, WorkerFailure>;
/// Trusted adapter; implementation requires real backend qualification.
pub trait ProjectionTarget: Send {
    /// Owned validated request; no document borrow or native transaction escapes preparation.
    type Request: Send + 'static;
    /// Fixed immutable mapping/model and deployment profile.
    fn profile(&self) -> &ProjectionProfile;
    /// Fixed approved physical generation.
    fn physical_target(&self) -> &str;
    /// Pure preparation: enforce complete wire, numeric, metric and profile bounds before intent.
    fn prepare(&self, documents: &[&ApprovedDocument]) -> crate::Result<Self::Request>;
    /// Apply the prepared fenced request and retrieve actual stored state, not HTTP acknowledgements.
    /// Transport must bound parsing/deadlines and fence old requests surviving caller cancellation.
    fn apply(
        &mut self,
        request: Self::Request,
    ) -> impl Future<Output = std::result::Result<Vec<RemoteObservation>, TargetFailure>> + Send;
}
/// Trusted source using public authorized Runtime::journal with a fixed export actor/profile.
pub trait ProjectionHistory: Send {
    /// Immutable mapping/model profile; changing grants requires generation invalidation.
    fn profile(&self) -> &ProjectionProfile;
    /// Fetch a bounded public authorized batch at this exact cursor, preserving history gaps.
    fn fetch(
        &mut self,
        after: &JournalCursor,
    ) -> impl Future<Output = crate::Result<JournalBatch>> + Send;
    /// Pure deterministic approved mapping; host vectors must already be available without I/O.
    fn document(&mut self, event: &JournalView) -> crate::Result<ApprovedDocument>;
}
impl From<Error> for WorkerFailure {
    fn from(e: Error) -> Self {
        Self::Core(e)
    }
}
impl From<StorageFailure> for WorkerFailure {
    fn from(e: StorageFailure) -> Self {
        Self::Storage(e)
    }
}
impl From<TargetFailure> for WorkerFailure {
    fn from(e: TargetFailure) -> Self {
        Self::Target(e)
    }
}
impl std::fmt::Display for WorkerFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("projection worker failure")
    }
}
impl std::error::Error for WorkerFailure {}
