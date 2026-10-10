use rom_sql_core::{ControlTableName, OperationDeadlines, OwnerError};
use std::time::Duration;
/// Separate host waiting, native statement and lock deadlines.
#[derive(Clone, Copy, Debug)]
pub struct Deadlines {
    pub(crate) connect: Duration,
    pub(crate) io: Duration,
    pub(crate) statement_ms: u32,
    pub(crate) lock_ms: u32,
}
impl Deadlines {
    /// Admit positive bounds, lock < statement < I/O, connect/I/O at most60seconds.
    pub fn new(
        connect: Duration,
        io: Duration,
        statement: Duration,
        lock: Duration,
    ) -> Result<Self, OwnerError> {
        let bounds = OperationDeadlines::new(connect, io)?;
        if lock.is_zero()
            || lock >= statement
            || statement >= io
            || !statement.subsec_nanos().is_multiple_of(1_000_000)
            || !lock.subsec_nanos().is_multiple_of(1_000_000)
        {
            return Err(OwnerError::Invalid);
        }
        Ok(Self {
            connect: bounds.connect(),
            io: bounds.io(),
            statement_ms: u32::try_from(statement.as_millis()).map_err(|_| OwnerError::Invalid)?,
            lock_ms: u32::try_from(lock.as_millis()).map_err(|_| OwnerError::Invalid)?,
        })
    }
}
/// Validated native control-table name and exact immutable store identity.
/// Provision its documented single-column-family singleton schema separately; no automatic DDL occurs.
#[derive(Clone)]
pub struct ControlTable {
    pub(crate) qualified: String,
    pub(crate) identity: [u8; 32],
}
impl ControlTable {
    /// Admit schema/table ASCII identifiers through63bytes, starting with a letter or underscore.
    pub fn new(schema: &str, table: &str, identity: [u8; 32]) -> Result<Self, OwnerError> {
        let name = ControlTableName::new(schema, table)?;
        Ok(Self {
            qualified: format!("\"{}\".\"{}\"", name.schema(), name.table()),
            identity,
        })
    }
}
impl std::fmt::Debug for ControlTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ControlTable { redacted }")
    }
}
