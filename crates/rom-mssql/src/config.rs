use rom_sql_core::OwnerError;
use std::time::Duration;
/// Explicit connect, whole-operation I/O and native lock deadlines.
#[derive(Clone, Copy, Debug)]
pub struct Deadlines {
    pub(crate) connect: Duration,
    pub(crate) io: Duration,
    pub(crate) lock_ms: i32,
}
impl Deadlines {
    /// Require positive deadlines at most 60 seconds; lock must be shorter than I/O.
    /// Lock granularity is milliseconds; sub-millisecond and fractional values are rejected.
    pub fn new(connect: Duration, io: Duration, lock: Duration) -> Result<Self, OwnerError> {
        let max = Duration::from_secs(60);
        if connect.is_zero()
            || io.is_zero()
            || connect > max
            || io > max
            || lock.is_zero()
            || lock >= io
            || lock > max
            || !lock.as_nanos().is_multiple_of(1_000_000)
        {
            return Err(OwnerError::Invalid);
        }
        Ok(Self {
            connect,
            io,
            lock_ms: i32::try_from(lock.as_millis()).map_err(|_| OwnerError::Invalid)?,
        })
    }
}
/// Fixed host-selected control table and exact immutable store identity.
/// No methods create, migrate or repair this table. Debug never exposes its identity.
#[derive(Clone)]
pub struct ControlTable {
    pub(crate) qualified: String,
    pub(crate) identity: [u8; 32],
}
impl ControlTable {
    /// Admit ASCII SQL identifiers of 1..=63 bytes and an opaque exact 32-byte identity.
    pub fn new(schema: &str, table: &str, identity: [u8; 32]) -> Result<Self, OwnerError> {
        for value in [schema, table] {
            if value.is_empty()
                || value.len() > 63
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || !value.as_bytes()[0].is_ascii_alphabetic() && value.as_bytes()[0] != b'_'
            {
                return Err(OwnerError::Invalid);
            }
        }
        Ok(Self {
            qualified: format!("[{schema}].[{table}]"),
            identity,
        })
    }
}
impl std::fmt::Debug for ControlTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ControlTable { redacted }")
    }
}
