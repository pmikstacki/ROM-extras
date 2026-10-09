//! Current host query approval and authoritative public Resource hydration.
use crate::{
    ApprovedTextQuery, ProjectionProfile, Result, SearchFailure, SearchPolicy, SearchScope,
    SearchTarget, TargetFailure, TextQuery, limits,
};
use rom::{Actor, ProjectedView, Runtime};
/// Fixed actor/kind/provider coordinator returning only current public projected values.
pub struct Search<T: SearchTarget, P: SearchPolicy> {
    runtime: Runtime,
    actor: Actor,
    kind: String,
    target: T,
    policy: P,
    profile: ProjectionProfile,
    physical: String,
}
impl<T: SearchTarget, P: SearchPolicy> Search<T, P> {
    /// Bind current authority to an immutable provider generation.
    pub fn new(runtime: Runtime, actor: Actor, kind: &str, target: T, policy: P) -> Result<Self> {
        limits::identifier(kind, 128)?;
        limits::identifier(target.physical_target(), 128)?;
        let profile = target.profile().clone();
        let physical = target.physical_target().into();
        Ok(Self {
            runtime,
            actor,
            kind: kind.into(),
            target,
            policy,
            profile,
            physical,
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
                kind: self.kind.clone(),
                profile: self.profile.clone(),
                physical: self.physical.clone(),
                query,
            },
        };
        self.authorize(&approved.scope).await?;
        let outcome = self.target.candidates(&approved).await;
        // Revoke disclosure even when the completed backend request failed.
        self.authorize(&approved.scope).await?;
        self.check_identity()?;
        let candidates = outcome.map_err(SearchFailure::Target)?;
        if candidates.len() > approved.query().candidate_budget() {
            return Err(SearchFailure::Target(TargetFailure::Rejected));
        }
        let mut unique = std::collections::BTreeSet::new();
        for candidate in &candidates {
            if candidate.key().kind != self.kind
                || !unique.insert((&candidate.key().kind, &candidate.key().id))
            {
                return Err(SearchFailure::Target(TargetFailure::Rejected));
            }
        }
        let mut views = Vec::new();
        for candidate in candidates {
            let view = match self
                .runtime
                .read_projected(&self.actor, &self.kind, &candidate.key().id)
                .await
            {
                Ok(view) => view,
                Err(rom::Error::Missing | rom::Error::Denied) => continue,
                Err(_) => return Err(SearchFailure::SourceUnavailable),
            };
            if &view.key != candidate.key()
                || view.revision != candidate.revision()
                || !view
                    .value
                    .as_ref()
                    .is_some_and(|v| v.contains_key(approved.scope().field()))
            {
                continue;
            }
            views.push(view);
            if views.len() == approved.query().result_limit() {
                break;
            }
        }
        self.authorize(&approved.scope).await?;
        Ok(views)
    }
    fn check_identity(&self) -> std::result::Result<(), SearchFailure> {
        if self.target.profile() != &self.profile || self.target.physical_target() != self.physical
        {
            return Err(SearchFailure::Core(crate::Error::Conflict));
        }
        Ok(())
    }
    async fn authorize(&mut self, scope: &SearchScope) -> std::result::Result<(), SearchFailure> {
        if !self.policy.authorize(&self.actor, scope).await {
            return Err(SearchFailure::Denied);
        }
        Ok(())
    }
}
