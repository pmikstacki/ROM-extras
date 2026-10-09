use crate::{Error, Limits};
use rom::{Invocation, Key, Operation, RevisionCondition, SourcePermit, SourceProvenance};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Trusted host selection of an ordinary action and its original receipt identity.
/// Debug and serialization are deliberately absent.
pub struct ActionBinding {
    /// Exact Resource kind and identifier; neither is normalized.
    pub target: Key,
    /// Registered domain action name.
    pub action: String,
    /// Original expected target revision.
    pub expected: Option<u64>,
    /// Original durable idempotency label.
    pub idempotency: String,
    /// Original explicit retry namespace.
    pub retry_epoch: u64,
}
/// Host-approved source authority, prepared before content is fetched.
/// This value is not an untrusted request or browser descriptor.
pub struct SourceGrant {
    /// Registered complete-Resource source owner.
    pub source: String,
    /// Host-managed nonzero requested generation.
    pub generation: u64,
    /// Host-attested origins for every final output field, not input fields.
    pub output_origins: BTreeMap<String, String>,
    /// Original managed control revision dependency.
    pub condition: RevisionCondition,
    /// Host-approved permit expiry in ROM's time units.
    pub valid_until: u64,
    /// Expected SHA-256 of exact document bytes; this is not producer authentication.
    pub expected_sha256: [u8; 32],
}
/// Immutable action and grant binding. Construction grants no runtime access.
pub struct ActionPlan {
    binding: ActionBinding,
    permit: SourcePermit,
    digest: [u8; 32],
    limits: Limits,
}
impl std::fmt::Debug for ActionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ActionPlan { redacted }")
    }
}
impl ActionPlan {
    /// Construct only after the native host authorizes the managed source and action.
    /// Runtime independently checks current service authority, dependencies and field coverage.
    pub fn trusted(
        binding: ActionBinding,
        grant: SourceGrant,
        limits: Limits,
    ) -> Result<Self, Error> {
        if binding.action.is_empty()
            || binding.action.len() > 256
            || binding.idempotency.is_empty()
            || binding.idempotency.len() > 256
            || binding.expected == Some(0)
        {
            return Err(Error::InvalidGrant);
        }
        let version = format!("sha256:{:x}", Sha256Digest(grant.expected_sha256));
        let permit = SourcePermit::trusted(
            binding.target.clone(),
            SourceProvenance {
                source: grant.source,
                version,
                generation: grant.generation,
                field_origins: grant.output_origins,
            },
            grant.condition,
            grant.valid_until,
        )
        .map_err(|_| Error::InvalidGrant)?;
        Ok(Self {
            binding,
            permit,
            digest: grant.expected_sha256,
            limits,
        })
    }
    /// Verify exact bounded bytes and prepare immutable ordinary action input.
    /// No fetch, mutation, automatic retry or actor selection occurs.
    pub fn prepare(&self, bytes: &[u8]) -> Result<PreparedAction, Error> {
        self.limits.check_bytes(bytes)?;
        let actual: [u8; 32] = Sha256::digest(bytes).into();
        if actual != self.digest {
            return Err(Error::DigestMismatch);
        }
        let input = self.limits.parse(bytes)?;
        Ok(PreparedAction {
            invocation: Invocation {
                retry_epoch: self.binding.retry_epoch,
                kind: self.binding.target.kind.clone(),
                id: self.binding.target.id.clone(),
                expected: self.binding.expected,
                idempotency: self.binding.idempotency.clone(),
                operation: Operation::Action {
                    name: self.binding.action.clone(),
                    input,
                },
            },
            permit: self.permit.clone(),
        })
    }
}
struct Sha256Digest([u8; 32]);
impl std::fmt::LowerHex for Sha256Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}
/// Retained original request. The host must preserve its actor scope for recovery.
/// An expired or superseded grant may prevent replay even after a commit.
pub struct PreparedAction {
    invocation: Invocation,
    permit: SourcePermit,
}
impl std::fmt::Debug for PreparedAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedAction { redacted }")
    }
}
impl PreparedAction {
    /// Clone the unchanged pair for `Runtime::invoke_sourced`.
    /// This does not renew authority or establish the outcome of a prior attempt.
    pub fn request(&self) -> (Invocation, SourcePermit) {
        (self.invocation.clone(), self.permit.clone())
    }
}
