//! Current vector policy and trusted provider capabilities, separate from text contracts.
use crate::{
    ApprovedVectorQuery, ProjectionProfile, SearchCandidate, TargetFailure, VectorMetric,
    VectorSearchScope,
};
use rom::Actor;
use std::future::Future;
/// Trusted current query/model authorization. Implementations must use bounded authoritative checks.
pub trait VectorSearchPolicy: Send {
    /// Approve the exact model, selected input fields, metric and generation without leaking query input.
    fn authorize(
        &mut self,
        actor: &Actor,
        scope: &VectorSearchScope,
    ) -> impl Future<Output = bool> + Send;
}
/// Trusted provider of bounded actual dense-vector candidates; no Resource disclosure authority is implied.
pub trait VectorSearchTarget: Send {
    /// Immutable deployment and mapping/model identity.
    fn profile(&self) -> &ProjectionProfile;
    /// Fixed physical generation.
    fn physical_target(&self) -> &str;
    /// Fixed finite vector dimensions.
    fn dimensions(&self) -> usize;
    /// Explicit configured metric; must remain unchanged throughout the query.
    fn metric(&self) -> VectorMetric;
    /// Query only after core approval; return original keys and ROM revisions without values or scores.
    fn candidates(
        &mut self,
        query: &ApprovedVectorQuery,
    ) -> impl Future<Output = std::result::Result<Vec<SearchCandidate>, TargetFailure>> + Send;
}
