//! Immutable selected public-view output; construction grants no export authority.
use crate::{
    Error, OperationMetadata, ProjectionProfile, Result, codec, document_encoding, limits,
};
use rom::{JournalView, Map, Value};
use sha2::{Digest, Sha256};
/// Immutable field selector and optional fixed-model vector dimension.
pub struct DocumentMapping {
    profile: ProjectionProfile,
    fields: Vec<String>,
    dimension: Option<usize>,
}
/// Immutable approved representation with core-derived exact content identity.
/// This contains sensitive approved values; Debug is sanitized and getters are explicit.
pub struct ApprovedDocument {
    profile: ProjectionProfile,
    metadata: OperationMetadata,
    fields: Option<Map<String, Value>>,
    vector: Option<Vec<f32>>,
}
impl DocumentMapping {
    /// Validate at most 64 unique fields and a model-bound dimension of 1 through 4096.
    pub fn new(
        mut profile: ProjectionProfile,
        mut fields: Vec<String>,
        dimension: Option<usize>,
    ) -> Result<Self> {
        if fields.len() > 64 {
            return Err(Error::TooLarge);
        }
        for f in &fields {
            limits::identifier(f, 128)?;
        }
        fields.sort();
        if fields.windows(2).any(|p| p[0] == p[1]) {
            return Err(Error::Conflict);
        }
        if dimension.is_some() != profile.model.is_some() {
            return Err(Error::Invalid);
        }
        if dimension.is_some_and(|d| d == 0 || d > 4096) {
            return Err(Error::Invalid);
        }
        let mut identity = Sha256::new();
        identity.update(b"ROM-extras/projection-mapping/v1");
        frame(&mut identity, &codec::profile(&profile));
        configuration(&mut identity, &fields, dimension);
        profile.mapping = identity
            .finalize()
            .iter()
            .flat_map(|byte| {
                let hex = b"0123456789abcdef";
                [
                    hex[(byte >> 4) as usize] as char,
                    hex[(byte & 15) as usize] as char,
                ]
            })
            .collect();
        Ok(Self {
            profile,
            fields,
            dimension,
        })
    }
    /// Select fields from an authorized historical view and validate its host-produced vector.
    /// Values have 16-KiB canonical, 4096-node and 32-level limits; vector bytes are separately bounded.
    /// Provider wire limits, export authority and backend numeric compatibility require separate checks.
    pub fn document(
        &self,
        event: &JournalView,
        mut vector: Option<Vec<f32>>,
    ) -> Result<ApprovedDocument> {
        self.validate_identity(event)?;
        self.validate_vector(event.view.value.is_none(), &mut vector)?;
        self.build(event, |_| Ok(vector))
    }
    fn validate_identity(&self, event: &JournalView) -> Result<()> {
        limits::key(&event.view.key)?;
        if event.position == 0 || event.view.revision == 0 {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    fn validate_vector(&self, tombstone: bool, vector: &mut Option<Vec<f32>>) -> Result<()> {
        match (tombstone, self.dimension, vector.as_mut()) {
            (true, _, None) | (false, None, None) => {}
            (false, Some(d), Some(v)) if v.len() == d => {
                for x in v {
                    if !x.is_finite() {
                        return Err(Error::Invalid);
                    }
                    if *x == 0.0 {
                        *x = 0.0;
                    }
                }
            }
            _ => return Err(Error::Invalid),
        }
        Ok(())
    }
    // One selected-value admission/identity implementation; providers see only bounded references.
    pub(crate) fn build(
        &self,
        event: &JournalView,
        generate: impl FnOnce(&[(&str, &Value)]) -> Result<Option<Vec<f32>>>,
    ) -> Result<ApprovedDocument> {
        self.validate_identity(event)?;
        let tombstone = event.view.value.is_none();
        // Gather only bounded references; clone selected trees only after complete validation.
        let selected: Vec<_> = self
            .fields
            .iter()
            .filter_map(|f| {
                event
                    .view
                    .value
                    .as_ref()
                    .and_then(|m| m.get(f))
                    .map(|v| (f.as_str(), v))
            })
            .collect();
        let encoded = if tombstone {
            Vec::new()
        } else {
            document_encoding::fields(&selected)?
        };
        let mut vector = generate(&selected)?;
        self.validate_vector(tombstone, &mut vector)?;
        let mut hash = Sha256::new();
        hash.update(b"ROM-extras/projection-document/v1");
        frame(&mut hash, &codec::profile(&self.profile));
        configuration(&mut hash, &self.fields, self.dimension);
        frame(&mut hash, event.view.key.kind.as_bytes());
        frame(&mut hash, event.view.key.id.as_bytes());
        hash.update(event.view.revision.to_be_bytes());
        hash.update([u8::from(tombstone)]);
        frame(&mut hash, &encoded);
        hash.update([u8::from(vector.is_some())]);
        if let Some(v) = &vector {
            for x in v {
                hash.update(x.to_bits().to_be_bytes());
            }
        }
        let metadata = OperationMetadata::new(
            event.view.key.clone(),
            event.position,
            event.view.revision,
            tombstone,
            hash.finalize().into(),
        )?;
        let fields = (!tombstone).then(|| {
            selected
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.clone()))
                .collect()
        });
        Ok(ApprovedDocument {
            profile: self.profile.clone(),
            metadata,
            fields,
            vector,
        })
    }
    /// Derived durable profile. Use this profile to create/open checkpoints and configure providers.
    pub fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    /// Immutable selected top-level field names in canonical order.
    pub fn selected_fields(&self) -> &[String] {
        &self.fields
    }
    /// Fixed vector dimension, absent for a non-vector mapping.
    pub fn vector_dimension(&self) -> Option<usize> {
        self.dimension
    }
}
fn frame(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u32).to_be_bytes());
    hash.update(bytes);
}
fn configuration(hash: &mut Sha256, fields: &[String], dimension: Option<usize>) {
    hash.update((fields.len() as u32).to_be_bytes());
    for field in fields {
        frame(hash, field.as_bytes());
    }
    hash.update((dimension.unwrap_or(0) as u32).to_be_bytes());
}
impl ApprovedDocument {
    /// Immutable original-key operation metadata, including the derived digest.
    pub fn metadata(&self) -> &OperationMetadata {
        &self.metadata
    }
    /// Selected approved values; absent for a tombstone.
    pub fn fields(&self) -> Option<&Map<String, Value>> {
        self.fields.as_ref()
    }
    /// Fixed-model finite vector; absent for non-vector mappings and tombstones.
    pub fn vector(&self) -> Option<&[f32]> {
        self.vector.as_deref()
    }
    /// Profile bound into this representation's content identity.
    pub fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
}
impl std::fmt::Debug for ApprovedDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApprovedDocument")
    }
}
impl std::fmt::Debug for DocumentMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DocumentMapping")
    }
}
