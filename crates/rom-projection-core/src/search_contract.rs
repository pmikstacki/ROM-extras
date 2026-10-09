//! Trusted asynchronous search policy and transport seams with fixed diagnostics.
use crate::{
    ApprovedTextQuery, Error, ProjectionProfile, SearchCandidate, SearchScope, TargetFailure,
};
use rom::Actor;
use std::future::Future;
/// Host-owned current authorization. Implementations must use bounded authoritative checks.
pub trait SearchPolicy: Send {
    /// Check current permission without leaking protected input in diagnostics.
    fn authorize(
        &mut self,
        actor: &Actor,
        scope: &SearchScope,
    ) -> impl Future<Output = bool> + Send;
}
/// Trusted provider. Pure identity getters and bounded actual candidate transport are required.
pub trait SearchTarget: Send {
    /// Immutable configured mapping/deployment identity.
    fn profile(&self) -> &ProjectionProfile;
    /// Fixed configured physical generation.
    fn physical_target(&self) -> &str;
    /// Return only bounded original keys and indexed revisions for a core-approved query.
    fn candidates(
        &mut self,
        query: &ApprovedTextQuery,
    ) -> impl Future<Output = std::result::Result<Vec<SearchCandidate>, TargetFailure>> + Send;
}
/// Fixed classification without query text, values, scores or backend error bodies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchFailure {
    /// Invalid core configuration or input.
    Core(Error),
    /// Current query policy denied this scope.
    Denied,
    /// Backend did not establish acceptable candidates.
    Target(TargetFailure),
    /// Public Runtime could not establish current projected values.
    SourceUnavailable,
}
impl std::fmt::Display for SearchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("projection search failure")
    }
}
impl std::error::Error for SearchFailure {}
