//! Typed core-approved fixed-generation search requests.
use crate::{OpenSearch, search_response};
use reqwest::Method;
use rom_projection_core::{
    ApprovedTextQuery, ProjectionProfile, SearchCandidate, SearchTarget, TargetFailure, TextMode,
};
use serde_json::json;
impl SearchTarget for OpenSearch {
    fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    fn physical_target(&self) -> &str {
        &self.physical
    }
    async fn candidates(
        &mut self,
        query: &ApprovedTextQuery,
    ) -> Result<Vec<SearchCandidate>, TargetFailure> {
        if query.profile() != &self.profile
            || query.physical_target() != self.physical
            || !self
                .text_fields
                .iter()
                .any(|field| field == query.query().field())
        {
            return Err(TargetFailure::Rejected);
        }
        let deadline = self.transport.deadline;
        let operation = async {
            self.verify_generation().await?;
            let mode = match query.query().mode() {
                TextMode::AllTerms => "and",
                TextMode::AnyTerms => "or",
            };
            let field = format!("rom_search.{}", query.query().field());
            let body = json!({"size":query.query().candidate_budget(),"track_total_hits":false,"version":true,
                "timeout":format!("{}ms", self.transport.deadline.as_millis()),
                "_source":["rom_kind","rom_id","rom_revision","rom_profile","rom_live"],
                "query":{"bool":{"filter":[{"term":{"rom_live":true}},{"term":{"rom_kind":query.scope().kind()}},{"term":{"rom_profile":self.profile_digest}}],
                    "must":[{"match":{field:{"query":query.query().text(),"operator":mode,"zero_terms_query":"none"}}}]}}});
            let (_, response) = self
                .transport
                .request(
                    Method::POST,
                    &[&self.physical, "_search"],
                    &[("allow_partial_search_results", "false")],
                    Some(serde_json::to_vec(&body).map_err(|_| TargetFailure::Rejected)?),
                    false,
                )
                .await?;
            let candidates = search_response::parse(
                &response,
                &self.physical,
                &self.profile_digest,
                query.scope().kind(),
                query.query().candidate_budget(),
            )?;
            self.verify_generation().await?;
            Ok(candidates)
        };
        tokio::time::timeout(deadline, operation)
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
}
