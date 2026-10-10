use crate::Cause;
use rom_blob::{Digest, ObjectKey};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};

/// Host-selected inventory bounds, validated before provider calls.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum entries, including zero-byte objects.
    pub max_entries: usize,
    /// Maximum bytes in one whole-object operation.
    pub max_object_bytes: usize,
    /// Maximum aggregate listed object bytes.
    pub max_total_bytes: u64,
    /// Maximum encoded or input JSON bytes.
    pub max_json_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_object_bytes: 64 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
            max_json_bytes: 4 * 1024 * 1024,
        }
    }
}
/// Exact opaque physical ROM key, content digest and expected length; not a logical Resource identifier.
#[derive(Clone)]
pub struct Entry {
    key: ObjectKey,
    digest: Digest,
    bytes: u64,
}
impl Entry {
    /// Prepare an entry. The containing Manifest enforces aggregate and object limits.
    pub fn new(key: ObjectKey, digest: Digest, bytes: u64) -> Self {
        Self { key, digest, bytes }
    }
    /// Exact unchanged public ROM key.
    pub fn key(&self) -> &ObjectKey {
        &self.key
    }
    /// Content identity, independent of the physical key and provider ETags.
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    /// Expected whole-object length in bytes.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}
impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("bytes", &self.bytes)
            .finish_non_exhaustive()
    }
}
/// Validated host-approved inventory. SHA-256 binds bytes, not authority or completeness.
#[derive(Clone)]
pub struct Manifest {
    checkpoint: Digest,
    entries: Vec<Entry>,
    limits: Limits,
    total: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: u32,
    checkpoint: String,
    entries: Vec<WireEntry>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireEntry {
    key: String,
    digest: String,
    bytes: u64,
}
impl Manifest {
    /// Validate all entries and encoded size before any provider I/O.
    pub fn new(checkpoint: Digest, entries: Vec<Entry>, limits: Limits) -> Result<Self, Cause> {
        if entries.len() > limits.max_entries {
            return Err(Cause::Limit);
        }
        let mut seen = BTreeSet::new();
        let mut total = 0u64;
        for entry in &entries {
            if entry.bytes > limits.max_object_bytes as u64 {
                return Err(Cause::Limit);
            }
            if !seen.insert(entry.key.as_str()) {
                return Err(Cause::Invalid);
            }
            total = total.checked_add(entry.bytes).ok_or(Cause::Limit)?;
            if total > limits.max_total_bytes {
                return Err(Cause::Limit);
            }
        }
        let m = Self {
            checkpoint,
            entries,
            limits,
            total,
        };
        m.encode()?;
        Ok(m)
    }
    /// Decode a bounded version-one JSON inventory. Unknown fields are rejected.
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Cause> {
        if bytes.len() > limits.max_json_bytes {
            return Err(Cause::Limit);
        }
        let w: Wire = serde_json::from_slice(bytes).map_err(|_| Cause::Invalid)?;
        if w.version != 1 {
            return Err(Cause::Invalid);
        }
        let checkpoint = Digest::parse(&w.checkpoint).map_err(|_| Cause::Invalid)?;
        let entries = w
            .entries
            .into_iter()
            .map(|e| {
                Ok(Entry::new(
                    ObjectKey::parse(&e.key).map_err(|_| Cause::Invalid)?,
                    Digest::parse(&e.digest).map_err(|_| Cause::Invalid)?,
                    e.bytes,
                ))
            })
            .collect::<Result<Vec<_>, Cause>>()?;
        Self::new(checkpoint, entries, limits)
    }
    /// Encode private inventory data. The host must protect the resulting bytes.
    pub fn encode(&self) -> Result<Vec<u8>, Cause> {
        let w = Wire {
            version: 1,
            checkpoint: self.checkpoint.as_str().into(),
            entries: self
                .entries
                .iter()
                .map(|e| WireEntry {
                    key: e.key.as_str().into(),
                    digest: e.digest.as_str().into(),
                    bytes: e.bytes,
                })
                .collect(),
        };
        let b = serde_json::to_vec(&w).map_err(|_| Cause::Invalid)?;
        if b.len() > self.limits.max_json_bytes {
            return Err(Cause::Limit);
        }
        Ok(b)
    }
    /// Checkpoint identifier for explicit host comparison; never authenticates an inventory.
    pub fn checkpoint(&self) -> &Digest {
        &self.checkpoint
    }
    /// Validated entries in host-supplied order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    /// Aggregate expected bytes.
    pub fn total_bytes(&self) -> u64 {
        self.total
    }
}
impl fmt::Debug for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Manifest")
            .field("entries", &self.entries.len())
            .field("bytes", &self.total)
            .finish_non_exhaustive()
    }
}
