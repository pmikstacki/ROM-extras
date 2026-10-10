/// Count-only progress for a verified prefix of a supplied manifest.
/// This is in-memory evidence, not a deployment or durable checkpoint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Entries whose downloaded bytes matched the manifest.
    pub verified: usize,
    /// Aggregate verified bytes.
    pub bytes: u64,
    /// Verified entries created and acknowledged by this invocation.
    pub created: usize,
    /// Verified entries already present, including concurrent-create conflicts.
    pub existing: usize,
}
