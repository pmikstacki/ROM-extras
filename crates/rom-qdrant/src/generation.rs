//! Host-owned concrete generations; metadata is not native collection identity.
use crate::writes::hex;
use rom_projection_core::{Error, ProjectionProfile, Result};
use serde_json::{Value, json};
/// Native distance metric. Cosine requires its separately selected representation profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distance {
    /// Inner product without upload normalization.
    Dot,
    /// Euclidean distance without upload normalization.
    Euclid,
    /// Manhattan distance without upload normalization.
    Manhattan,
    /// Automatic native normalization; refused by the exact-bit profile.
    Cosine,
}
impl Distance {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Dot => "Dot",
            Self::Euclid => "Euclid",
            Self::Manhattan => "Manhattan",
            Self::Cosine => "Cosine",
        }
    }
}
/// Fixed host-selected generation. Its name must never be reused or restored with copied markers.
/// This contract assumes exclusive host administration while any prepared work remains pending.
#[derive(Clone, PartialEq, Eq)]
pub struct Generation {
    pub(crate) profile: ProjectionProfile,
    pub(crate) physical: String,
    pub(crate) nonce: String,
    pub(crate) dimensions: usize,
    pub(crate) distance: Distance,
}
impl Generation {
    /// Bind a concrete name, host nonce, 1..=4096 dimensions and exact float32 metric.
    /// No credentials, authorization, model execution or remote operation is performed.
    pub fn new(
        profile: ProjectionProfile,
        physical: &str,
        nonce: &str,
        dimensions: usize,
        distance: Distance,
    ) -> Result<Self> {
        for s in [physical, nonce] {
            if s.len() > 128 {
                return Err(Error::TooLarge);
            }
            if s.is_empty()
                || !s
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            {
                return Err(Error::Invalid);
            }
        }
        if !(1..=4096).contains(&dimensions) {
            return Err(Error::Invalid);
        }
        if distance == Distance::Cosine {
            return Err(Error::Unsupported);
        }
        Ok(Self {
            profile,
            physical: physical.into(),
            nonce: nonce.into(),
            dimensions,
            distance,
        })
    }
    /// Select general nonzero finite Cosine preparation and exact Qdrant1.19.2 readback candidates.
    /// Hosts must qualify the native build and floating-point environment; zero vectors are refused.
    pub fn cosine_v1_19_2(
        profile: ProjectionProfile,
        physical: &str,
        nonce: &str,
        dimensions: usize,
    ) -> Result<Self> {
        let mut generation = Self::new(profile, physical, nonce, dimensions, Distance::Dot)?;
        generation.distance = Distance::Cosine;
        Ok(generation)
    }
    /// Pure administrator provisioning document. Send it through a separate trusted host path.
    /// The writer cannot create collections. The host must exclude aliases and name reuse.
    pub fn definition(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&json!({
            "vectors":{"embedding":{"size":self.dimensions,"distance":self.distance.name(),"datatype":"float32"}},
            "shard_number":1,"replication_factor":1,"write_consistency_factor":1,
            "metadata":self.marker()
        })).map_err(|_| Error::Invalid)
    }
    pub(crate) fn marker(&self) -> Value {
        json!({"rom_extras_projection":{"format":1,"profile":hex(&self.profile.fingerprint()),
            "generation":self.nonce,"normalization":if self.distance==Distance::Cosine {"cosine-f64-qdrant-1.19.2-v1"} else {"none"}}})
    }
}
