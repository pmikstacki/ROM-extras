//! Fixed dense query under one total budget, with native identity checks before and after.
use crate::{
    Distance, Qdrant, search_response,
    writes::{hex, point_id},
};
use reqwest::Method;
use rom_projection_core::{
    ApprovedVectorQuery, ProjectionProfile, SearchCandidate, TargetFailure, VectorMetric,
    VectorSearchTarget,
};
use serde_json::json;
impl VectorSearchTarget for Qdrant {
    fn profile(&self) -> &ProjectionProfile {
        &self.generation.profile
    }
    fn physical_target(&self) -> &str {
        &self.generation.physical
    }
    fn dimensions(&self) -> usize {
        self.generation.dimensions
    }
    fn metric(&self) -> VectorMetric {
        match self.generation.distance {
            Distance::Dot => VectorMetric::Dot,
            Distance::Euclid => VectorMetric::Euclid,
            Distance::Manhattan => VectorMetric::Manhattan,
            Distance::Cosine => VectorMetric::Cosine,
        }
    }
    async fn candidates(
        &mut self,
        query: &ApprovedVectorQuery,
    ) -> Result<Vec<SearchCandidate>, TargetFailure> {
        if query.profile() != &self.generation.profile
            || query.physical_target() != self.generation.physical
            || query.scope().metric() != self.metric()
            || query.query().vector().len() != self.generation.dimensions
        {
            return Err(TargetFailure::Rejected);
        }
        let operation = async {
            self.inspect_generation().await?;
            let fingerprint = hex(&self.generation.profile.fingerprint());
            let mut filter = json!({"must":[
                {"key":"rom_kind","match":{"value":query.scope().kind()}},
                {"key":"rom_profile","match":{"value":fingerprint}},
                {"key":"rom_live","match":{"value":true}}
            ]});
            if let Some(key) = query.query().excluded() {
                filter["must_not"] = json!([{"has_id":[point_id(&key.kind, &key.id)]}]);
            }
            let body = serde_json::to_vec(&json!({
                "query":query.query().vector(), "using":"embedding", "filter":filter,
                "params":{"exact":true}, "limit":query.query().candidate_budget(), "offset":0,
                "with_vector":false,
                "with_payload":["rom_kind","rom_id","rom_profile","rom_revision_hi","rom_revision_lo","rom_live"]
            })).map_err(|_| TargetFailure::Rejected)?;
            let timeout = self.http.deadline().as_secs_f64().ceil().max(1.) as u64;
            let timeout = timeout.to_string();
            let (_, response) = self
                .http
                .request(
                    Method::POST,
                    &["collections", &self.generation.physical, "points", "query"],
                    &[("consistency", "1"), ("timeout", &timeout)],
                    Some(body),
                    false,
                )
                .await?;
            let candidates = search_response::parse(
                query.scope().kind(),
                &fingerprint,
                query.query(),
                self.generation.distance,
                &response,
            )?;
            self.inspect_generation().await?;
            Ok(candidates)
        };
        tokio::time::timeout(self.http.deadline(), operation)
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
}
