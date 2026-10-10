use crate::{Binding, Counts, Error, Inventory, Limits};
use rom_blob::{Blob, BlobState, Digest, ObjectKey};
use rom_blob_recovery::Entry;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: u32,
    backend: rom_backup::Backend,
    checkpoint: String,
    counts: Counts,
    bindings: Vec<WireBinding>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBinding {
    id: String,
    revision: u64,
    upload_revision: u64,
    owner: String,
    store: String,
    key: String,
    digest: String,
    bytes: u64,
}
impl WireBinding {
    fn from_row(id: &str, revision: u64, blob: &Blob, key: &str) -> Self {
        Self {
            id: id.into(),
            revision,
            upload_revision: blob.upload_revision,
            owner: blob.owner.clone(),
            store: blob.store.clone(),
            key: key.into(),
            digest: blob.digest.as_str().into(),
            bytes: blob.bytes,
        }
    }
}
pub(crate) fn check_plan_size(
    backend: rom_backup::Backend,
    checkpoint: &Digest,
    counts: Counts,
    rows: &[crate::inspection::ReadyRow],
    max: usize,
) -> Result<(), Error> {
    let mut size = serde_json::to_vec(&Wire {
        version: 1,
        backend,
        checkpoint: checkpoint.as_str().into(),
        counts,
        bindings: Vec::new(),
    })
    .map_err(|_| Error::Invalid)?
    .len();
    if size > max {
        return Err(Error::TooLarge);
    }
    // Public physical keys have 64 lowercase hex bytes. Measure length without minting a binding.
    let key = "0000000000000000000000000000000000000000000000000000000000000000";
    for (index, row) in rows.iter().enumerate() {
        let bytes = serde_json::to_vec(&WireBinding::from_row(
            &row.id,
            row.revision,
            &row.blob,
            key,
        ))
        .map_err(|_| Error::Invalid)?;
        size = size
            .checked_add(bytes.len())
            .and_then(|n| n.checked_add(usize::from(index > 0)))
            .ok_or(Error::TooLarge)?;
        if size > max {
            return Err(Error::TooLarge);
        }
    }
    Ok(())
}
impl Inventory {
    /// Encode sensitive operational data. Protect these bytes separately from Debug reports.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let bindings = self
            .bindings
            .iter()
            .map(|b| WireBinding::from_row(&b.id, b.revision, &b.blob, b.entry.key().as_str()))
            .collect();
        let bytes = serde_json::to_vec(&Wire {
            version: 1,
            backend: self.backend,
            checkpoint: self.checkpoint.as_str().into(),
            counts: self.counts,
            bindings,
        })
        .map_err(|_| Error::Invalid)?;
        if bytes.len() > self.limits.max_json_bytes {
            return Err(Error::TooLarge);
        }
        Ok(bytes)
    }
    /// Validate bounded version-one JSON. This does not authenticate checkpoint provenance or completeness.
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        if bytes.len() > limits.max_json_bytes {
            return Err(Error::TooLarge);
        }
        let w: Wire = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        if w.version != 1 {
            return Err(Error::Unsupported);
        }
        if w.bindings.len() > limits.max_ready {
            return Err(Error::TooLarge);
        }
        let checkpoint = Digest::parse(&w.checkpoint).map_err(|_| Error::Invalid)?;
        let bindings = w
            .bindings
            .into_iter()
            .map(|b| {
                let digest = Digest::parse(&b.digest).map_err(|_| Error::Invalid)?;
                let key = ObjectKey::parse(&b.key).map_err(|_| Error::Invalid)?;
                Ok(Binding {
                    id: b.id,
                    revision: b.revision,
                    blob: Blob {
                        owner: b.owner,
                        store: b.store,
                        digest: digest.clone(),
                        bytes: b.bytes,
                        state: BlobState::Ready,
                        upload_revision: b.upload_revision,
                    },
                    entry: Entry::new(key, digest, b.bytes),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Self::new(w.backend, checkpoint, w.counts, bindings, limits)
    }
}
