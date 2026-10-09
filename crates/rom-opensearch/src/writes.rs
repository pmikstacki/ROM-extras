//! Immutable wire preparation and exact native stored-source reconciliation.
use crate::{OpenSearch, mapping, transport::WIRE_LIMIT};
use reqwest::Method;
use rom_projection_core::{
    ApprovedDocument, Error, OperationMetadata, ProjectionProfile, ProjectionTarget,
    RemoteObservation, Result, TargetFailure,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
/// Owned bounded wire request. Its private metadata is bound to the preparing target.
pub struct PreparedPage {
    pub(crate) profile: ProjectionProfile,
    pub(crate) physical: String,
    pub(crate) body: Vec<u8>,
    pub(crate) entries: Vec<Entry>,
}
pub(crate) struct Entry {
    id: String,
    value: Value,
    metadata: OperationMetadata,
}
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(crate) fn id(kind: &str, key: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"ROM-extras/opensearch-key/v1");
    for s in [kind, key] {
        hash.update((s.len() as u32).to_be_bytes());
        hash.update(s.as_bytes());
    }
    hex(&hash.finalize())
}
impl ProjectionTarget for OpenSearch {
    type Request = PreparedPage;
    fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    fn physical_target(&self) -> &str {
        &self.physical
    }
    fn prepare(&self, documents: &[&ApprovedDocument]) -> Result<PreparedPage> {
        if documents.len() > 64 {
            return Err(Error::TooLarge);
        }
        let mut body = Vec::new();
        let mut entries = Vec::new();
        let mut ids = BTreeSet::new();
        for doc in documents {
            if doc.profile() != &self.profile {
                return Err(Error::Conflict);
            }
            if doc.vector().is_some() {
                return Err(Error::Unsupported);
            }
            let m = doc.metadata();
            if m.revision() > i64::MAX as u64 {
                return Err(Error::Unsupported);
            }
            let id = id(&m.key().kind, &m.key().id);
            if !ids.insert(id.clone()) {
                return Err(Error::Conflict);
            }
            let mut search = serde_json::Map::new();
            if let Some(values) = doc.fields() {
                for field in &self.text_fields {
                    if let Some(v) = values.get(field) {
                        if !v.is_string() && !v.is_null() {
                            return Err(Error::Unsupported);
                        }
                        search.insert(field.clone(), v.clone());
                    }
                }
            }
            let mut value = json!({"rom_profile":self.profile_digest,"rom_kind":m.key().kind,"rom_id":m.key().id,"rom_revision":m.revision(),"rom_digest":hex(m.digest()),"rom_live":!m.is_tombstone()});
            if let Some(fields) = doc.fields() {
                value["rom_values"] = json!(fields);
                value["rom_search"] = json!(search);
            }
            for line in [
                json!({"index":{"_id":id,"version":m.revision(),"version_type":"external"}}),
                value.clone(),
            ] {
                let encoded = serde_json::to_vec(&line).map_err(|_| Error::Invalid)?;
                if encoded.len() + 1 > WIRE_LIMIT - body.len() {
                    return Err(Error::TooLarge);
                }
                body.extend(encoded);
                body.push(b'\n');
            }
            entries.push(Entry {
                id,
                value,
                metadata: m.clone(),
            });
        }
        Ok(PreparedPage {
            profile: self.profile.clone(),
            physical: self.physical.clone(),
            body,
            entries,
        })
    }
    async fn apply(
        &mut self,
        request: PreparedPage,
    ) -> std::result::Result<Vec<RemoteObservation>, TargetFailure> {
        if request.profile != self.profile || request.physical != self.physical {
            return Err(TargetFailure::Rejected);
        }
        let deadline = self.transport.deadline;
        let operation = async {
            self.verify_generation().await?;
            if request.entries.is_empty() {
                return Ok(Vec::new());
            }
            let (_, bulk) = self
                .transport
                .request(
                    Method::POST,
                    &[&self.physical, "_bulk"],
                    &[("refresh", "false")],
                    Some(request.body),
                    true,
                )
                .await?;
            let items = bulk["items"]
                .as_array()
                .filter(|a| a.len() == request.entries.len())
                .ok_or(TargetFailure::Unknown)?;
            let errors = bulk["errors"].as_bool().ok_or(TargetFailure::Unknown)?;
            let mut actual_errors = false;
            for (item, entry) in items.iter().zip(&request.entries) {
                let object = item
                    .as_object()
                    .filter(|o| o.len() == 1)
                    .ok_or(TargetFailure::Unknown)?;
                let native = object.get("index").ok_or(TargetFailure::Unknown)?;
                if native["_index"].as_str() != Some(&self.physical)
                    || native["_id"].as_str() != Some(&entry.id)
                {
                    return Err(TargetFailure::Unknown);
                }
                match native["status"].as_u64() {
                    Some(200 | 201) => {
                        if native["_version"].as_u64() != Some(entry.metadata.revision())
                            || native.get("error").is_some()
                        {
                            return Err(TargetFailure::Unknown);
                        }
                    }
                    Some(409) => {
                        actual_errors = true;
                    }
                    Some(400..=499) => return Err(TargetFailure::Rejected),
                    _ => return Err(TargetFailure::Unknown),
                }
            }
            if errors != actual_errors {
                return Err(TargetFailure::Unknown);
            }
            self.stored_observations(&request.entries).await
        };
        tokio::time::timeout(deadline, operation)
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
}
impl OpenSearch {
    /// Inspect every actual stored source without dispatching writes or advancing a checkpoint.
    /// The prepared page must match this adapter's fixed profile and physical generation.
    pub async fn inspect_prepared(
        &mut self,
        request: &PreparedPage,
    ) -> std::result::Result<Vec<RemoteObservation>, TargetFailure> {
        if request.profile != self.profile || request.physical != self.physical {
            return Err(TargetFailure::Rejected);
        }
        let deadline = self.transport.deadline;
        let operation = async {
            self.verify_generation().await?;
            self.stored_observations(&request.entries).await
        };
        tokio::time::timeout(deadline, operation)
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
    async fn stored_observations(
        &self,
        entries: &[Entry],
    ) -> std::result::Result<Vec<RemoteObservation>, TargetFailure> {
        let mut observations = Vec::new();
        for entry in entries {
            let (_, stored) = self
                .transport
                .request(
                    Method::GET,
                    &[&self.physical, "_doc", &entry.id],
                    &[("realtime", "true"), ("preference", "_primary")],
                    None,
                    false,
                )
                .await?;
            if stored["found"].as_bool() != Some(true)
                || stored["_index"].as_str() != Some(&self.physical)
                || stored["_id"].as_str() != Some(&entry.id)
                || stored["_version"].as_u64() != Some(entry.metadata.revision())
                || stored["_source"] != entry.value
            {
                return Err(TargetFailure::Rejected);
            }
            observations.push(
                RemoteObservation::new(&self.profile, &self.physical, entry.metadata.clone())
                    .map_err(|_| TargetFailure::Rejected)?,
            );
        }
        Ok(observations)
    }

    /// Create a fresh fixed generation with request translog durability and explicit mappings.
    /// A lost acknowledgement is unknown; inspect the existing generation rather than assuming absence.
    pub async fn create_generation(&mut self) -> std::result::Result<(), TargetFailure> {
        let deadline = self.transport.deadline;
        tokio::time::timeout(deadline, self.create_native_generation())
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
    async fn create_native_generation(&mut self) -> std::result::Result<(), TargetFailure> {
        let definition = json!({"settings":{"number_of_shards":1,"number_of_replicas":0,"index.translog.durability":"request"},"mappings":mapping::definition(&self.profile_digest,&self.text_fields)});
        let (_, response) = self
            .transport
            .request(
                Method::PUT,
                &[&self.physical],
                &[],
                Some(serde_json::to_vec(&definition).map_err(|_| TargetFailure::Rejected)?),
                false,
            )
            .await?;
        if response["acknowledged"].as_bool() != Some(true) {
            return Err(TargetFailure::Unknown);
        }
        self.inspect_generation().await
    }
    /// Inspect exact mappings, request durability, a concrete physical index and fixed index UUID.
    pub async fn verify_generation(&mut self) -> std::result::Result<(), TargetFailure> {
        let deadline = self.transport.deadline;
        tokio::time::timeout(deadline, self.inspect_generation())
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
    async fn inspect_generation(&mut self) -> std::result::Result<(), TargetFailure> {
        let (_, response) = self
            .transport
            .request(Method::GET, &[&self.physical, "_mapping"], &[], None, false)
            .await?;
        let object = response
            .as_object()
            .filter(|o| o.len() == 1)
            .ok_or(TargetFailure::Rejected)?;
        let index = object.get(&self.physical).ok_or(TargetFailure::Rejected)?;
        if index["mappings"] != mapping::definition(&self.profile_digest, &self.text_fields) {
            return Err(TargetFailure::Rejected);
        }
        let (_, settings) = self
            .transport
            .request(
                Method::GET,
                &[&self.physical, "_settings"],
                &[],
                None,
                false,
            )
            .await?;
        let object = settings
            .as_object()
            .filter(|o| o.len() == 1)
            .ok_or(TargetFailure::Rejected)?;
        let index = object.get(&self.physical).ok_or(TargetFailure::Rejected)?;
        if index["settings"]["index"]["translog"]["durability"].as_str() != Some("request") {
            return Err(TargetFailure::Rejected);
        }
        let uuid = index["settings"]["index"]["uuid"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 128)
            .ok_or(TargetFailure::Rejected)?;
        if self.index_uuid.as_ref().is_some_and(|old| old != uuid) {
            return Err(TargetFailure::Rejected);
        }
        self.index_uuid = Some(uuid.into());
        Ok(())
    }
}
