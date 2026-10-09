//! Native response validation and inspect-only reconciliation under one operation budget.
use crate::{PreparedPage, Qdrant, page::Entry};
use reqwest::Method;
use rom_projection_core::{RemoteObservation, TargetFailure};
use serde_json::{Value, json};
type NativeResult<T> = std::result::Result<T, TargetFailure>;
pub(crate) fn completed(value: &Value) -> NativeResult<()> {
    if value["status"].as_str() != Some("ok")
        || value["result"]["status"].as_str() != Some("completed")
        || value["result"]["operation_id"].as_u64().is_none()
    {
        return Err(TargetFailure::Unknown);
    }
    Ok(())
}
pub(crate) fn same_point(expected: &Value, actual: &Value) -> NativeResult<()> {
    if actual["id"] != expected["id"] || actual["payload"] != expected["payload"] {
        return Err(TargetFailure::Rejected);
    }
    let e = expected["vector"]
        .as_object()
        .ok_or(TargetFailure::Rejected)?;
    let a = actual["vector"]
        .as_object()
        .ok_or(TargetFailure::Rejected)?;
    if e.len() != a.len() {
        return Err(TargetFailure::Rejected);
    }
    for (name, vector) in e {
        let e = vector.as_array().ok_or(TargetFailure::Rejected)?;
        let a = a
            .get(name)
            .and_then(Value::as_array)
            .filter(|a| a.len() == e.len())
            .ok_or(TargetFailure::Rejected)?;
        for (e, a) in e.iter().zip(a) {
            let e = e.as_f64().ok_or(TargetFailure::Rejected)? as f32;
            let a = a.as_f64().ok_or(TargetFailure::Rejected)? as f32;
            if !a.is_finite() || e.to_bits() != a.to_bits() {
                return Err(TargetFailure::Rejected);
            }
        }
    }
    Ok(())
}
impl Qdrant {
    pub(crate) async fn apply_page(
        &self,
        request: PreparedPage,
    ) -> NativeResult<Vec<RemoteObservation>> {
        if request.generation != self.generation {
            return Err(TargetFailure::Rejected);
        }
        let operation = async {
            self.inspect_generation().await?;
            for entry in &request.entries {
                let (_, value) = self
                    .http
                    .request(
                        Method::PUT,
                        &["collections", &self.generation.physical, "points"],
                        &[("wait", "true"), ("ordering", "strong")],
                        Some(entry.body.clone()),
                        false,
                    )
                    .await?;
                completed(&value)?;
            }
            self.stored(&request.entries).await
        };
        tokio::time::timeout(self.http.deadline(), operation)
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
    /// Inspect actual points without another write or checkpoint change.
    /// A mismatched prepared generation is refused before any network operation.
    pub async fn inspect_prepared(
        &self,
        request: &PreparedPage,
    ) -> NativeResult<Vec<RemoteObservation>> {
        if request.generation != self.generation {
            return Err(TargetFailure::Rejected);
        }
        let operation = async {
            self.inspect_generation().await?;
            self.stored(&request.entries).await
        };
        tokio::time::timeout(self.http.deadline(), operation)
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
    async fn stored(&self, entries: &[Entry]) -> NativeResult<Vec<RemoteObservation>> {
        let mut observations = Vec::new();
        for entry in entries {
            let body = serde_json::to_vec(
                &json!({"ids":[entry.point["id"]],"with_payload":true,"with_vector":true}),
            )
            .map_err(|_| TargetFailure::Rejected)?;
            let (_, value) = self
                .http
                .request(
                    Method::POST,
                    &["collections", &self.generation.physical, "points"],
                    &[],
                    Some(body),
                    false,
                )
                .await?;
            if value["status"].as_str() != Some("ok") {
                return Err(TargetFailure::Unknown);
            }
            let points = value["result"].as_array().ok_or(TargetFailure::Unknown)?;
            if points.len() != 1 {
                return Err(TargetFailure::Rejected);
            }
            same_point(&entry.point, &points[0])?;
            observations.push(
                RemoteObservation::new(
                    &self.generation.profile,
                    &self.generation.physical,
                    entry.metadata.clone(),
                )
                .map_err(|_| TargetFailure::Rejected)?,
            );
        }
        self.inspect_generation().await?;
        Ok(observations)
    }
    /// Verify fixed metadata and native configuration. This does not establish an immutable native UUID.
    pub async fn verify_generation(&self) -> NativeResult<()> {
        tokio::time::timeout(self.http.deadline(), self.inspect_generation())
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
    pub(crate) async fn inspect_generation(&self) -> NativeResult<()> {
        let (_, value) = self
            .http
            .request(
                Method::GET,
                &["collections", &self.generation.physical],
                &[],
                None,
                false,
            )
            .await?;
        if value["status"].as_str() != Some("ok") {
            return Err(TargetFailure::Unknown);
        }
        let config = &value["result"]["config"];
        let params = &config["params"];
        let vectors = params["vectors"]
            .as_object()
            .filter(|o| o.len() == 1)
            .ok_or(TargetFailure::Rejected)?;
        let vector = vectors.get("embedding").ok_or(TargetFailure::Rejected)?;
        if config["metadata"] != self.generation.marker()
            || params["shard_number"].as_u64() != Some(1)
            || params["replication_factor"].as_u64() != Some(1)
            || params["write_consistency_factor"].as_u64() != Some(1)
            || vector["size"].as_u64() != Some(self.generation.dimensions as u64)
            || vector["distance"].as_str() != Some(self.generation.distance.name())
            || vector["datatype"].as_str() != Some("float32")
            || !vector["multivector_config"].is_null()
            || !vector["quantization_config"].is_null()
            || !config["quantization_config"].is_null()
            || !(params["sparse_vectors"].is_null() || params["sparse_vectors"] == json!({}))
        {
            return Err(TargetFailure::Rejected);
        }
        // A scoped reader can inspect aliases for its collection. Refuse alias-addressed targets.
        let (_, aliases) = self
            .http
            .request(
                Method::GET,
                &["collections", &self.generation.physical, "aliases"],
                &[],
                None,
                false,
            )
            .await?;
        if aliases["status"].as_str() != Some("ok") {
            return Err(TargetFailure::Unknown);
        }
        let aliases = aliases["result"]["aliases"]
            .as_array()
            .ok_or(TargetFailure::Unknown)?;
        if aliases
            .iter()
            .any(|a| a["alias_name"].as_str() == Some(&self.generation.physical))
        {
            return Err(TargetFailure::Rejected);
        }
        Ok(())
    }
}
