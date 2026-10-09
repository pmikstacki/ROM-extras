//! Bounded sensitive vector input and private core approval.
use crate::{Error, ProjectionProfile, Result, limits};
use rom::Key;
/// Explicit vector metric; provider qualification determines which metrics are available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorMetric {
    /// Inner-product similarity, ordered from larger to smaller.
    Dot,
    /// Euclidean distance, ordered from smaller to larger.
    Euclid,
    /// L1 distance, ordered from smaller to larger.
    Manhattan,
    /// Normalized inner-product similarity; requires provider-specific representation qualification.
    Cosine,
}
/// Immutable finite float32 input. Construction does not grant query or model authority.
pub struct VectorQuery {
    vector: Vec<f32>,
    limit: usize,
    budget: usize,
    excluded: Option<Key>,
}
impl VectorQuery {
    /// Validate 1..=4096 components, 1..=64 results and result-limit..=256 candidates.
    /// Negative zero is canonicalized; no metric normalization or model inference occurs.
    pub fn new(mut vector: Vec<f32>, result_limit: usize, candidate_budget: usize) -> Result<Self> {
        if vector.len() > 4096 {
            return Err(Error::TooLarge);
        }
        if vector.is_empty()
            || vector.iter().any(|v| !v.is_finite())
            || result_limit == 0
            || result_limit > 64
            || candidate_budget < result_limit
            || candidate_budget > 256
        {
            return Err(Error::Invalid);
        }
        for value in &mut vector {
            if *value == 0. {
                *value = 0.;
            }
        }
        Ok(Self {
            vector,
            limit: result_limit,
            budget: candidate_budget,
            excluded: None,
        })
    }
    /// Exclude this exact original key. The coordinator additionally requires its fixed Resource kind.
    pub fn excluding(mut self, key: Key) -> Result<Self> {
        limits::key(&key)?;
        self.excluded = Some(key);
        Ok(self)
    }
    /// Explicit sensitive access, without exposing input through Debug or diagnostics.
    pub fn vector(&self) -> &[f32] {
        &self.vector
    }
    /// Maximum current authorized Resources returned.
    pub fn result_limit(&self) -> usize {
        self.limit
    }
    /// Maximum native candidates admitted from one response.
    pub fn candidate_budget(&self) -> usize {
        self.budget
    }
    /// Optional exact original Resource exclusion; not an implicit native ID-based query.
    pub fn excluded(&self) -> Option<&Key> {
        self.excluded.as_ref()
    }
}
/// Current host authorization scope, with all disclosure fields derived from the bound DocumentMapping.
pub struct VectorSearchScope {
    pub(crate) kind: String,
    pub(crate) profile: ProjectionProfile,
    pub(crate) physical: String,
    pub(crate) model: String,
    pub(crate) fields: Vec<String>,
    pub(crate) metric: VectorMetric,
    pub(crate) query: VectorQuery,
}
impl VectorSearchScope {
    /// Fixed original Resource kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }
    /// Immutable deployment and mapping/model identity.
    pub fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    /// Fixed physical generation.
    pub fn physical_target(&self) -> &str {
        &self.physical
    }
    /// Exact configured embedding model identity.
    pub fn model_id(&self) -> &str {
        &self.model
    }
    /// All selected Resource inputs which must remain disclosed by current public projection.
    pub fn fields(&self) -> &[String] {
        &self.fields
    }
    /// Configured native metric.
    pub fn metric(&self) -> VectorMetric {
        self.metric
    }
    /// Validated query dimension, matching the configured target and mapping.
    pub fn dimensions(&self) -> usize {
        self.query.vector.len()
    }
    /// Maximum current authorized output.
    pub fn result_limit(&self) -> usize {
        self.query.limit
    }
    /// Maximum admitted native candidates.
    pub fn candidate_budget(&self) -> usize {
        self.query.budget
    }
    /// Optional exact original key exclusion.
    pub fn excluded(&self) -> Option<&Key> {
        self.query.excluded()
    }
}
/// Core-minted typed query. Only the coordinator constructs it after current authorization.
///
/// ```compile_fail
/// use rom_projection_core::{ApprovedVectorQuery,VectorSearchScope};
/// fn forge(scope:VectorSearchScope)->ApprovedVectorQuery { ApprovedVectorQuery {scope} }
/// ```
pub struct ApprovedVectorQuery {
    pub(crate) scope: VectorSearchScope,
}
impl ApprovedVectorQuery {
    /// Current host scope bound to this request.
    pub fn scope(&self) -> &VectorSearchScope {
        &self.scope
    }
    /// Bounded sensitive query input; access is explicit.
    pub fn query(&self) -> &VectorQuery {
        &self.scope.query
    }
    /// Immutable mapping/model and deployment identity.
    pub fn profile(&self) -> &ProjectionProfile {
        self.scope.profile()
    }
    /// Fixed physical generation.
    pub fn physical_target(&self) -> &str {
        self.scope.physical_target()
    }
}
crate::diagnostics::sanitized_debug!(VectorQuery, VectorSearchScope, ApprovedVectorQuery);
