//! Validated native ownership state, independent of ROM Work and Resource revisions.
/// Finite ownership failures; no native error messages or credentials are retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerError {
    /// Invalid token, generation or owner-state representation.
    Invalid,
    /// The immutable native format is unsupported; no writes are permitted.
    UnsupportedFormat,
    /// Explicit provisioning has not established the native control row.
    Uninitialized,
    /// A different active owner prevents an idle claim.
    Busy,
    /// The expected generation, token or state no longer matches.
    Stale,
    /// The generation cannot advance without exceeding the protocol range.
    Exhausted,
    /// The operation could not complete; commit uncertainty must use Unknown instead.
    Unavailable,
    /// Native commit or rollback outcome was not established.
    Unknown,
}
impl std::fmt::Display for OwnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for OwnerError {}
/// Exact 32-byte incarnation token supplied by the trusted host. Debug is redacted.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OwnerToken([u8; 32]);
impl OwnerToken {
    /// Admit a nonzero fixed-size token. The host supplies fresh unpredictable bytes.
    pub fn new(bytes: [u8; 32]) -> Result<Self, OwnerError> {
        if bytes == [0; 32] {
            return Err(OwnerError::Invalid);
        }
        Ok(Self(bytes))
    }
    /// Exact native bind bytes; protect these from logs and public disclosure.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
impl std::fmt::Debug for OwnerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OwnerToken { redacted }")
    }
}
/// Validated persisted control state. Inspection alone does not grant ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerState {
    generation: u64,
    token: Option<OwnerToken>,
}
impl OwnerState {
    /// Admit a generation in 0..=i64::MAX. Generation zero must be idle.
    pub fn new(generation: u64, token: Option<OwnerToken>) -> Result<Self, OwnerError> {
        if generation > i64::MAX as u64 || generation == 0 && token.is_some() {
            return Err(OwnerError::Invalid);
        }
        Ok(Self { generation, token })
    }
    /// Native protocol generation; distinct from ROM's full-u64 Resource revisions.
    pub fn generation(self) -> u64 {
        self.generation
    }
    /// Exact current native owner token, if any. This is trusted recovery material.
    pub fn token(self) -> Option<OwnerToken> {
        self.token
    }
    pub(crate) fn advance(self, token: OwnerToken) -> Result<Ownership, OwnerError> {
        if self.generation == i64::MAX as u64 {
            return Err(OwnerError::Exhausted);
        }
        Ok(Ownership(Self {
            generation: self.generation + 1,
            token: Some(token),
        }))
    }
}
/// Acknowledged ownership transition. This is a native fence, not an authorization grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ownership(pub(crate) OwnerState);
impl Ownership {
    /// Acknowledged generation.
    pub fn generation(self) -> u64 {
        self.0.generation
    }
    /// Exact persisted ownership expected by protected native transactions.
    pub fn state(self) -> OwnerState {
        self.0
    }
}
