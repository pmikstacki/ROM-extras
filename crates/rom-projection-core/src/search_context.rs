//! Shared candidate admission and current public hydration for text and vector queries.
use crate::{
    Error, ProjectionProfile, Result, SearchCandidate, SearchFailure, TargetFailure, limits,
};
use rom::{Actor, Key, ProjectedView, Runtime};
pub(crate) struct SearchContext {
    pub(crate) runtime: Runtime,
    pub(crate) actor: Actor,
    pub(crate) kind: String,
    pub(crate) profile: ProjectionProfile,
    pub(crate) physical: String,
}
impl SearchContext {
    pub(crate) fn new(
        runtime: Runtime,
        actor: Actor,
        kind: &str,
        profile: &ProjectionProfile,
        physical: &str,
    ) -> Result<Self> {
        limits::identifier(kind, 128)?;
        limits::identifier(physical, 128)?;
        Ok(Self {
            runtime,
            actor,
            kind: kind.into(),
            profile: profile.clone(),
            physical: physical.into(),
        })
    }
    pub(crate) fn check_identity(
        &self,
        profile: &ProjectionProfile,
        physical: &str,
    ) -> std::result::Result<(), SearchFailure> {
        if profile != &self.profile || physical != self.physical {
            return Err(SearchFailure::Core(Error::Conflict));
        }
        Ok(())
    }
    pub(crate) async fn hydrate(
        &self,
        candidates: Vec<SearchCandidate>,
        budget: usize,
        limit: usize,
        fields: &[&str],
        excluded: Option<&Key>,
    ) -> std::result::Result<Vec<ProjectedView>, SearchFailure> {
        if candidates.len() > budget {
            return Err(SearchFailure::Target(TargetFailure::Rejected));
        }
        let mut unique = std::collections::BTreeSet::new();
        for candidate in &candidates {
            if candidate.key().kind != self.kind
                || excluded == Some(candidate.key())
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
                    .is_some_and(|v| fields.iter().all(|f| v.contains_key(*f)))
            {
                continue;
            }
            views.push(view);
            if views.len() == limit {
                break;
            }
        }
        Ok(views)
    }
}
