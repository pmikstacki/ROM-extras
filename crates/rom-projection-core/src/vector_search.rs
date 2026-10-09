//! Core approval and current authoritative public hydration for dense vector candidates.
use crate::{
    ApprovedVectorQuery, DocumentMapping, Error, Result, SearchFailure, VectorMetric, VectorQuery,
    VectorSearchPolicy, VectorSearchScope, VectorSearchTarget, search_context::SearchContext,
};
use rom::{Actor, ProjectedView, Runtime};
/// Fixed vector coordinator. All embedded input fields must survive current public field projection.
pub struct VectorSearch<T: VectorSearchTarget, P: VectorSearchPolicy> {
    context: SearchContext,
    target: T,
    policy: P,
    fields: Vec<String>,
    model: String,
    dimensions: usize,
    metric: VectorMetric,
}
impl<T: VectorSearchTarget, P: VectorSearchPolicy> VectorSearch<T, P> {
    /// Bind current host authority to the exact approved DocumentMapping and configured target shape.
    /// A different selector/profile or dimension is refused without a provider request.
    pub fn new(
        runtime: Runtime,
        actor: Actor,
        kind: &str,
        target: T,
        policy: P,
        mapping: DocumentMapping,
    ) -> Result<Self> {
        if target.profile() != mapping.profile() {
            return Err(Error::Conflict);
        }
        let dimensions = mapping.vector_dimension().ok_or(Error::Unsupported)?;
        if target.dimensions() != dimensions {
            return Err(Error::Conflict);
        }
        let model = mapping.profile().model.clone().ok_or(Error::Invalid)?;
        let context = SearchContext::new(
            runtime,
            actor,
            kind,
            target.profile(),
            target.physical_target(),
        )?;
        let metric = target.metric();
        Ok(Self {
            context,
            target,
            policy,
            fields: mapping.selected_fields().to_vec(),
            model,
            dimensions,
            metric,
        })
    }
    /// Authorize, query a bounded page, and return only current authorized projected Resources.
    /// Candidate omissions can underfill results; no global snapshot, recall or pagination guarantee is supplied.
    pub async fn execute(
        &mut self,
        query: VectorQuery,
    ) -> std::result::Result<Vec<ProjectedView>, SearchFailure> {
        self.check_identity()?;
        if query.vector().len() != self.dimensions
            || query
                .excluded()
                .is_some_and(|k| k.kind != self.context.kind)
        {
            return Err(SearchFailure::Core(Error::Invalid));
        }
        let approved = ApprovedVectorQuery {
            scope: VectorSearchScope {
                kind: self.context.kind.clone(),
                profile: self.context.profile.clone(),
                physical: self.context.physical.clone(),
                model: self.model.clone(),
                fields: self.fields.clone(),
                metric: self.metric,
                query,
            },
        };
        self.authorize(approved.scope()).await?;
        let outcome = self.target.candidates(&approved).await;
        // Current denial takes precedence even over an uncertain or rejected native outcome.
        self.authorize(approved.scope()).await?;
        self.check_identity()?;
        let candidates = outcome.map_err(SearchFailure::Target)?;
        let fields: Vec<_> = self.fields.iter().map(String::as_str).collect();
        let views = self
            .context
            .hydrate(
                candidates,
                approved.query().candidate_budget(),
                approved.query().result_limit(),
                &fields,
                approved.query().excluded(),
            )
            .await?;
        self.authorize(approved.scope()).await?;
        self.check_identity()?;
        Ok(views)
    }
    fn check_identity(&self) -> std::result::Result<(), SearchFailure> {
        self.context
            .check_identity(self.target.profile(), self.target.physical_target())?;
        if self.target.dimensions() != self.dimensions || self.target.metric() != self.metric {
            return Err(SearchFailure::Core(Error::Conflict));
        }
        Ok(())
    }
    async fn authorize(
        &mut self,
        scope: &VectorSearchScope,
    ) -> std::result::Result<(), SearchFailure> {
        if !self.policy.authorize(&self.context.actor, scope).await {
            return Err(SearchFailure::Denied);
        }
        Ok(())
    }
}
