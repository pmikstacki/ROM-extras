//! Current host text approval using shared authoritative public Resource hydration.
use crate::{
    ApprovedTextQuery, Result, SearchFailure, SearchPolicy, SearchScope, SearchTarget, TextQuery,
    search_context::SearchContext,
};
use rom::{Actor, ProjectedView, Runtime};
/// Fixed actor/kind/provider coordinator returning only current public projected values.
pub struct Search<T: SearchTarget, P: SearchPolicy> {
    context: SearchContext,
    target: T,
    policy: P,
}
impl<T: SearchTarget, P: SearchPolicy> Search<T, P> {
    /// Bind current authority to an immutable provider generation.
    pub fn new(runtime: Runtime, actor: Actor, kind: &str, target: T, policy: P) -> Result<Self> {
        let context = SearchContext::new(
            runtime,
            actor,
            kind,
            target.profile(),
            target.physical_target(),
        )?;
        Ok(Self {
            context,
            target,
            policy,
        })
    }
    /// Authorize, fetch one bounded candidate page and hydrate current authorized values.
    pub async fn execute(
        &mut self,
        query: TextQuery,
    ) -> std::result::Result<Vec<ProjectedView>, SearchFailure> {
        self.check_identity()?;
        let approved = ApprovedTextQuery {
            scope: SearchScope {
                kind: self.context.kind.clone(),
                profile: self.context.profile.clone(),
                physical: self.context.physical.clone(),
                query,
            },
        };
        self.authorize(&approved.scope).await?;
        let outcome = self.target.candidates(&approved).await;
        self.authorize(&approved.scope).await?;
        self.check_identity()?;
        let candidates = outcome.map_err(SearchFailure::Target)?;
        let views = self
            .context
            .hydrate(
                candidates,
                approved.query().candidate_budget(),
                approved.query().result_limit(),
                &[approved.scope().field()],
                None,
            )
            .await?;
        self.authorize(&approved.scope).await?;
        self.check_identity()?;
        Ok(views)
    }
    fn check_identity(&self) -> std::result::Result<(), SearchFailure> {
        self.context
            .check_identity(self.target.profile(), self.target.physical_target())
    }
    async fn authorize(&mut self, scope: &SearchScope) -> std::result::Result<(), SearchFailure> {
        if !self.policy.authorize(&self.context.actor, scope).await {
            return Err(SearchFailure::Denied);
        }
        Ok(())
    }
}
