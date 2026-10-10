use crate::{Error, Limits};
use rom::{Key, Resource};
use rom_blob::{Blob, BlobState, Digest};
use rom_blob_recovery::{Entry, Manifest};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};
/// Explicit current archive-row counts. Other Resource kinds are not inspected as external blobs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    /// Required current Ready Blob rows.
    pub ready: usize,
    /// Excluded pending reservations.
    pub pending: usize,
    /// Excluded detached Blob rows.
    pub detached: usize,
    /// Excluded built-in Blob tombstones.
    pub tombstones: usize,
    /// All rows of other Resource kinds, including their tombstones.
    pub other_rows: usize,
}
/// Private exact binding from one current Ready checkpoint row to one physical publication.
#[derive(Clone)]
pub struct Binding {
    pub(crate) id: String,
    pub(crate) revision: u64,
    pub(crate) blob: Blob,
    pub(crate) entry: Entry,
}
impl Binding {
    /// Exact original built-in Resource key. No normalization occurs.
    pub fn resource_key(&self) -> Key {
        Key {
            kind: Blob::KIND.into(),
            id: self.id.clone(),
        }
    }
    /// Current checkpoint Resource revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Original upload reservation revision used by the public service.
    pub fn upload_revision(&self) -> u64 {
        self.blob.upload_revision
    }
    /// Explicit private owner identity.
    pub fn owner(&self) -> &str {
        &self.blob.owner
    }
    /// Exact logical store alias, independent of provider credentials and endpoints.
    pub fn store(&self) -> &str {
        &self.blob.store
    }
    /// Physical key, separate content digest and byte length.
    pub fn entry(&self) -> &Entry {
        &self.entry
    }
}
impl fmt::Debug for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Binding")
            .field("bytes", &self.entry.bytes())
            .finish_non_exhaustive()
    }
}
/// Structurally validated private checkpoint inventory. Authenticity remains a host obligation.
pub struct Inventory {
    pub(crate) backend: rom_backup::Backend,
    pub(crate) checkpoint: Digest,
    pub(crate) counts: Counts,
    pub(crate) bindings: Vec<Binding>,
    pub(crate) limits: Limits,
}
impl Inventory {
    pub(crate) fn new(
        backend: rom_backup::Backend,
        checkpoint: Digest,
        counts: Counts,
        bindings: Vec<Binding>,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        if bindings.len() > limits.max_ready {
            return Err(Error::TooLarge);
        }
        if counts.ready != bindings.len() {
            return Err(Error::Invalid);
        }
        let rows = [
            counts.ready,
            counts.pending,
            counts.detached,
            counts.tombstones,
            counts.other_rows,
        ]
        .into_iter()
        .try_fold(0usize, |a, n| a.checked_add(n).ok_or(Error::TooLarge))?;
        if rows > limits.archive.max_records {
            return Err(Error::TooLarge);
        }
        let mut ids = BTreeSet::new();
        let mut keys = BTreeSet::new();
        let mut stores = BTreeSet::new();
        let mut total = 0u64;
        for binding in &bindings {
            validate_row(&binding.id, binding.revision, &binding.blob, limits)?;
            stores.insert(binding.store());
            if stores.len() > 32 {
                return Err(Error::TooLarge);
            }
            if binding.entry.digest() != &binding.blob.digest
                || binding.entry.bytes() != binding.blob.bytes
                || !ids.insert(binding.id.as_str())
                || !keys.insert((binding.store(), binding.entry.key().as_str()))
            {
                return Err(Error::Invalid);
            }
            total = total
                .checked_add(binding.blob.bytes)
                .ok_or(Error::TooLarge)?;
            if total > limits.max_total_bytes {
                return Err(Error::TooLarge);
            }
        }
        let i = Self {
            backend,
            checkpoint,
            counts,
            bindings,
            limits,
        };
        i.encode()?;
        Ok(i)
    }
    /// Exact archive backend; does not enable a disabled native feature.
    pub fn backend(&self) -> rom_backup::Backend {
        self.backend
    }
    /// SHA-256 of the trusted immutable source archive; not a signature.
    pub fn checkpoint(&self) -> &Digest {
        &self.checkpoint
    }
    /// Explicit counts and exclusions.
    pub fn counts(&self) -> Counts {
        self.counts
    }
    /// Explicit private Ready-row bindings in archive order.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }
    /// Prepare one existing recovery manifest. Never combine physical namespaces across aliases.
    pub fn for_store(&self, alias: &str) -> Result<Manifest, Error> {
        let entries: Vec<_> = self
            .bindings
            .iter()
            .filter(|b| b.store() == alias)
            .map(|b| b.entry.clone())
            .collect();
        if entries.is_empty() {
            return Err(Error::Missing);
        }
        Manifest::new(self.checkpoint.clone(), entries, self.limits.recovery()).map_err(Error::from)
    }
}
pub(crate) fn validate_row(
    id: &str,
    revision: u64,
    blob: &Blob,
    limits: Limits,
) -> Result<(), Error> {
    if id.is_empty()
        || blob.owner.is_empty()
        || blob.store.is_empty()
        || blob.upload_revision == 0
        || blob.upload_revision > revision
        || revision == 0
        || revision > i64::MAX as u64
        || blob.state != BlobState::Ready
    {
        return Err(Error::Invalid);
    }
    if id.len() > 4096
        || blob.owner.len() > 4096
        || blob.store.len() > 64
        || blob.bytes > limits.max_object_bytes as u64
    {
        return Err(Error::TooLarge);
    }
    Ok(())
}
pub(crate) fn same_blob(a: &Blob, b: &Blob) -> bool {
    a.owner == b.owner
        && a.store == b.store
        && a.digest == b.digest
        && a.bytes == b.bytes
        && a.state == b.state
        && a.upload_revision == b.upload_revision
}
impl fmt::Debug for Inventory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inventory")
            .field("counts", &self.counts)
            .finish_non_exhaustive()
    }
}
