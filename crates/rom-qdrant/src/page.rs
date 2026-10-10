//! Bound owned preparation; original conditional encoder remains the single source of wire semantics.
use crate::{Generation, PreparedWrite, Qdrant};
use rom_projection_core::{
    ApprovedDocument, Error, OperationMetadata, ProjectionProfile, ProjectionTarget,
    RemoteObservation, Result, TargetFailure,
};
use rom_projection_http::WIRE_LIMIT;
use serde_json::Value;
use std::collections::BTreeSet;
/// Owned page with no Debug disclosure. Its generation binding cannot be changed by a caller.
pub struct PreparedPage {
    pub(crate) generation: Generation,
    pub(crate) entries: Vec<Entry>,
}
pub(crate) struct Entry {
    pub(crate) body: Vec<u8>,
    pub(crate) point: Value,
    pub(crate) metadata: OperationMetadata,
    pub(crate) candidates: Option<Vec<Vec<f32>>>,
}
impl ProjectionTarget for Qdrant {
    type Request = PreparedPage;
    fn profile(&self) -> &ProjectionProfile {
        &self.generation.profile
    }
    fn physical_target(&self) -> &str {
        &self.generation.physical
    }
    fn prepare(&self, documents: &[&ApprovedDocument]) -> Result<PreparedPage> {
        if documents.len() > 64 {
            return Err(Error::TooLarge);
        }
        let mut entries = Vec::new();
        let mut ids = BTreeSet::new();
        let mut total = 0;
        for doc in documents {
            if doc.profile() != self.profile() {
                return Err(Error::Conflict);
            }
            if let Some(vector) = doc.vector()
                && (vector.len() != self.generation.dimensions
                    || vector.iter().any(|f| !f.is_finite()))
            {
                return Err(Error::Invalid);
            }
            let vector = if self.generation.distance == crate::Distance::Cosine {
                doc.vector().map(crate::cosine::normalize).transpose()?
            } else {
                None
            };
            let candidates = vector.as_deref().map(crate::cosine::candidates);
            let write = PreparedWrite::with_vector(doc, vector.as_deref().or(doc.vector()))?;
            let body = write.as_bytes().to_vec();
            if body.len() > WIRE_LIMIT - total {
                return Err(Error::TooLarge);
            }
            total += body.len();
            let value: Value = serde_json::from_slice(&body).map_err(|_| Error::Invalid)?;
            let point = value["points"][0].clone();
            let id = point["id"].as_str().ok_or(Error::Invalid)?;
            if !ids.insert(id.to_owned()) {
                return Err(Error::Conflict);
            }
            entries.push(Entry {
                body,
                point,
                metadata: doc.metadata().clone(),
                candidates,
            });
        }
        Ok(PreparedPage {
            generation: self.generation.clone(),
            entries,
        })
    }
    async fn apply(
        &mut self,
        request: PreparedPage,
    ) -> std::result::Result<Vec<RemoteObservation>, TargetFailure> {
        self.apply_page(request).await
    }
}
