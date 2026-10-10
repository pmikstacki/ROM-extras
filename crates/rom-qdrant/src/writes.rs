//! Original-key conditional point replacement, without local acknowledgement claims.
use rom_projection_core::{ApprovedDocument, Error, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
/// Owned conditional request; approved values are sensitive and Debug is omitted.
/// Construction does not qualify a remote generation or establish native acceptance.
pub struct PreparedWrite {
    body: Vec<u8>,
}
impl PreparedWrite {
    /// Prepare a single bounded whole-point replacement with strict revision and original-key fencing.
    /// Selected JSON is retained as a string to avoid native payload numeric coercion.
    pub fn new(document: &ApprovedDocument) -> Result<Self> {
        Self::with_vector(document, document.vector())
    }
    pub(crate) fn with_vector(document: &ApprovedDocument, vector: Option<&[f32]>) -> Result<Self> {
        let metadata = document.metadata();
        let hi = metadata.revision() >> 32;
        let lo = metadata.revision() & 0xffff_ffff;
        let mut payload = json!({
            "rom_kind":metadata.key().kind,
            "rom_id":metadata.key().id,
            "rom_profile":hex(&document.profile().fingerprint()),
            "rom_revision_hi":hi,
            "rom_revision_lo":lo,
            "rom_digest":hex(metadata.digest()),
            "rom_live":!metadata.is_tombstone(),
        });
        if let Some(fields) = document.fields() {
            payload["rom_values"] =
                json!(serde_json::to_string(fields).map_err(|_| Error::Invalid)?);
        }
        let vector = match vector {
            Some(vector) => json!({"embedding":vector}),
            None if metadata.is_tombstone() => json!({}),
            None => return Err(Error::Invalid),
        };
        let body = serde_json::to_vec(&json!({
            "points":[{
                "id":point_id(&metadata.key().kind,&metadata.key().id),
                "vector":vector,
                "payload":payload,
            }],
            "update_mode":"upsert",
            "update_filter":{"must":[
                {"key":"rom_kind","match":{"value":metadata.key().kind}},
                {"key":"rom_id","match":{"value":metadata.key().id}},
                {"should":[
                    {"key":"rom_revision_hi","range":{"lt":hi}},
                    {"must":[
                        {"key":"rom_revision_hi","match":{"value":hi}},
                        {"key":"rom_revision_lo","range":{"lt":lo}}
                    ]}
                ]}
            ]}
        }))
        .map_err(|_| Error::Invalid)?;
        if body.len() > 1_048_576 {
            return Err(Error::TooLarge);
        }
        Ok(Self { body })
    }
    /// Bounded JSON for the fixed-generation upsert endpoint.
    pub fn as_bytes(&self) -> &[u8] {
        &self.body
    }
}
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub(crate) fn point_id(kind: &str, id: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"ROM-extras/qdrant-key/v1");
    for part in [kind, id] {
        hash.update((part.len() as u32).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let digest = hash.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let h = hex(&bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}
