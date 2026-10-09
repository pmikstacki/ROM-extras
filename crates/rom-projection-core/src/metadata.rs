//! Validated bounded metadata and exact provider reconciliation.
use crate::{Error, Result, codec, limits};
use rom::{JournalCursor, Key};
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "metadata_tests.rs"]
mod tests;

use crate::diagnostics::sanitized_debug;

/// Immutable deployment, provider and mapping/model identity.
#[derive(Clone, PartialEq, Eq)]
pub struct ProjectionProfile {
    pub(crate) deployment: String,
    pub(crate) provider: String,
    pub(crate) mapping: String,
    pub(crate) model: Option<String>,
}
impl ProjectionProfile {
    /// Stable SHA-256 identity of the complete deployment, provider, mapping and model profile.
    /// Providers use this identity in native generation metadata; it grants no export authority.
    pub fn fingerprint(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        Sha256::digest(codec::profile(self)).into()
    }
    /// Validate identity lengths without granting provider support or export authority.
    pub fn new(
        deployment: &str,
        provider: &str,
        mapping: &str,
        model: Option<&str>,
    ) -> Result<Self> {
        for value in [deployment, provider, mapping].into_iter().chain(model) {
            limits::identifier(value, limits::IDENTIFIER)?;
        }
        Ok(Self {
            deployment: deployment.into(),
            provider: provider.into(),
            mapping: mapping.into(),
            model: model.map(str::to_owned),
        })
    }
}

/// Physical target and durable per-kind public ROM cursors.
#[derive(Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub(crate) physical: String,
    pub(crate) cursors: Vec<JournalCursor>,
    pub(crate) sequence: u64,
}
impl Checkpoint {
    /// Validate a fresh sequence-zero checkpoint. No journal completeness is inferred.
    pub fn new(physical: &str, mut cursors: Vec<JournalCursor>) -> Result<Self> {
        limits::identifier(physical, limits::IDENTIFIER)?;
        if cursors.is_empty() {
            return Err(Error::Invalid);
        }
        if cursors.len() > limits::KINDS {
            return Err(Error::TooLarge);
        }
        for cursor in &cursors {
            limits::cursor(cursor)?;
        }
        cursors.sort_by(|a, b| a.kind.cmp(&b.kind));
        if cursors.windows(2).any(|pair| pair[0].kind == pair[1].kind) {
            return Err(Error::Conflict);
        }
        Ok(Self {
            physical: physical.into(),
            cursors,
            sequence: 0,
        })
    }
    /// Read a kind's current cursor without inventing an initial position.
    pub fn cursor(&self, kind: &str) -> Option<&JournalCursor> {
        self.cursors.iter().find(|cursor| cursor.kind == kind)
    }
    /// Durable transaction sequence, separate from every journal position.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Approved current backend target.
    pub fn physical_target(&self) -> &str {
        &self.physical
    }
}

/// Original key, selected event position and deterministic approved content identity.
#[derive(Clone, PartialEq, Eq)]
pub struct OperationMetadata {
    pub(crate) key: Key,
    pub(crate) position: u64,
    pub(crate) revision: u64,
    pub(crate) tombstone: bool,
    pub(crate) digest: [u8; 32],
}
impl OperationMetadata {
    /// Validate bounded identity and nonzero revision; a digest grants no export authority.
    pub fn new(
        key: Key,
        position: u64,
        revision: u64,
        tombstone: bool,
        digest: [u8; 32],
    ) -> Result<Self> {
        limits::key(&key)?;
        if revision == 0 || position == 0 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            key,
            position,
            revision,
            tombstone,
            digest,
        })
    }
    /// Original public ROM identity.
    pub fn key(&self) -> &Key {
        &self.key
    }
    /// Exact u64 Resource revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Whether the mapped state is a persistent tombstone.
    pub fn is_tombstone(&self) -> bool {
        self.tombstone
    }
    /// Host mapping digest, including the approved representation.
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
    pub(crate) fn key_state(&self) -> KeyState {
        KeyState {
            revision: self.revision,
            tombstone: self.tombstone,
            digest: self.digest,
        }
    }
}

/// Immutable bounded page accepted before network dispatch.
#[derive(Clone, PartialEq, Eq)]
pub struct PageIntent {
    pub(crate) expected: Checkpoint,
    pub(crate) next: JournalCursor,
    pub(crate) operations: Vec<OperationMetadata>,
}
impl PageIntent {
    /// Validate ordered input, then retain one highest revision per exact original key.
    pub fn new(
        expected: &Checkpoint,
        next: JournalCursor,
        operations: Vec<OperationMetadata>,
    ) -> Result<Self> {
        limits::cursor(&next)?;
        let previous = expected.cursor(&next.kind).ok_or(Error::HistoryGap)?;
        if previous.generation != next.generation || previous.position > next.position {
            return Err(Error::HistoryGap);
        }
        expected.sequence.checked_add(2).ok_or(Error::TooLarge)?;
        if operations.len() > limits::OPERATIONS {
            return Err(Error::TooLarge);
        }
        let mut position = previous.position;
        let mut collapsed: BTreeMap<Vec<u8>, OperationMetadata> = BTreeMap::new();
        for operation in operations {
            if operation.key.kind != next.kind
                || operation.position <= position
                || operation.position > next.position
            {
                return Err(Error::Invalid);
            }
            position = operation.position;
            let key = codec::key(&operation.key);
            if let Some(old) = collapsed.get(&key)
                && (operation.revision < old.revision
                    || (operation.revision == old.revision
                        && operation.key_state() != old.key_state()))
            {
                return Err(Error::Conflict);
            }
            collapsed.insert(key, operation);
        }
        let page = Self {
            expected: expected.clone(),
            next,
            operations: collapsed.into_values().collect(),
        };
        if codec::page(&page).len() > limits::RECORD {
            return Err(Error::TooLarge);
        }
        Ok(page)
    }
    /// Selected backend operations in canonical exact-key order.
    pub fn operations(&self) -> &[OperationMetadata] {
        &self.operations
    }
    /// Inspected page cursor, which can advance over filtered facts.
    pub fn next_cursor(&self) -> &JournalCursor {
        &self.next
    }
    /// Verify trusted adapter observations; HTTP success alone is not an observation.
    ///
    /// This compares metadata, not transport authenticity. Only qualified adapters
    /// may supply observations obtained through their verified backend transport.
    pub fn reconcile(
        &self,
        profile: &ProjectionProfile,
        observations: Vec<RemoteObservation>,
    ) -> Result<ReconciledPage> {
        if observations.len() > limits::OPERATIONS {
            return Err(Error::TooLarge);
        }
        if observations.len() != self.operations.len() {
            return Err(Error::Conflict);
        }
        let mut observed = BTreeMap::new();
        for observation in observations {
            if observation.profile != *profile || observation.physical != self.expected.physical {
                return Err(Error::Conflict);
            }
            if observed
                .insert(
                    codec::key(&observation.operation.key),
                    observation.operation.key_state(),
                )
                .is_some()
            {
                return Err(Error::Conflict);
            }
        }
        for operation in &self.operations {
            if observed.get(&codec::key(&operation.key)) != Some(&operation.key_state()) {
                return Err(Error::Conflict);
            }
        }
        Ok(ReconciledPage {
            fingerprint: codec::page_digest(profile, self),
        })
    }
}

/// A trusted adapter's actual stored state, not a write acknowledgement.
pub struct RemoteObservation {
    profile: ProjectionProfile,
    physical: String,
    operation: OperationMetadata,
}
impl RemoteObservation {
    /// Attach an actual observation to its inspected profile and target.
    ///
    /// The core rejects conflicting target/profile metadata during reconciliation.
    pub fn new(
        profile: &ProjectionProfile,
        physical: &str,
        operation: OperationMetadata,
    ) -> Result<Self> {
        limits::identifier(physical, limits::IDENTIFIER)?;
        Ok(Self {
            profile: profile.clone(),
            physical: physical.into(),
            operation,
        })
    }
}
/// Core-validated exact observations bound to one profile and complete page.
///
/// Callers cannot forge a completed-page acknowledgement:
/// ```compile_fail
/// let forged = rom_projection_core::ReconciledPage { fingerprint: [0; 32] };
/// ```
pub struct ReconciledPage {
    pub(crate) fingerprint: [u8; 32],
}

/// Durable last acknowledged original-key representation.
#[derive(Clone, PartialEq, Eq)]
pub struct KeyState {
    pub(crate) revision: u64,
    pub(crate) tombstone: bool,
    pub(crate) digest: [u8; 32],
}
impl KeyState {
    /// Last acknowledged exact revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Persistent tombstone status.
    pub fn is_tombstone(&self) -> bool {
        self.tombstone
    }
    /// Last acknowledged canonical mapping digest.
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// Durable control records inspected without an escaping native transaction.
#[derive(Clone, PartialEq, Eq)]
pub struct StoreSnapshot {
    pub(crate) checkpoint: Checkpoint,
    pub(crate) pending: Option<PageIntent>,
    pub(crate) completed: Option<(TransactionId, TransactionId)>,
    pub(crate) completed_page: Option<PageIntent>,
}
impl StoreSnapshot {
    /// Current checkpoint; a pending page has not advanced its cursor.
    pub fn checkpoint(&self) -> &Checkpoint {
        &self.checkpoint
    }
    /// Persisted intent that must resolve before later work.
    pub fn pending_page(&self) -> Option<&PageIntent> {
        self.pending.as_ref()
    }
}
/// A bounded transition token retained by the caller across acknowledgement loss.
#[derive(Clone, PartialEq, Eq)]
pub struct TransactionId {
    pub(crate) sequence: u64,
    pub(crate) transition: u8,
    pub(crate) fingerprint: [u8; 32],
    pub(crate) previous: [u8; 32],
}
impl TransactionId {
    /// Encode a caller-retained token without serializing private host data.
    pub fn encode(&self) -> [u8; 83] {
        codec::token(self)
    }
    /// Reject malformed token bytes before reconciliation or native I/O.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        codec::decode_token(bytes)
    }
}
/// Proven result of the retained exact transition after reopening.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitStatus {
    /// Durable records prove that the transition applied.
    Applied,
    /// Durable previous control state proves that the transition did not apply.
    Absent,
}
sanitized_debug!(
    ProjectionProfile,
    Checkpoint,
    OperationMetadata,
    PageIntent,
    RemoteObservation,
    ReconciledPage,
    KeyState,
    StoreSnapshot,
    TransactionId
);
