use crate::OwnerError;
use std::time::Duration;
/// Portable, validated schema/table identifiers for extras-owned native control rows.
/// Engines retain their own quoting and transaction contracts.
#[derive(Clone)]
pub struct ControlTableName {
    schema: String,
    table: String,
}
impl ControlTableName {
    /// Admit1..=63 ASCII alphanumeric/underscore bytes, starting with a letter or underscore.
    pub fn new(schema: &str, table: &str) -> Result<Self, OwnerError> {
        for value in [schema, table] {
            if value.is_empty()
                || value.len() > 63
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || !value
                    .as_bytes()
                    .first()
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
            {
                return Err(OwnerError::Invalid);
            }
        }
        Ok(Self {
            schema: schema.into(),
            table: table.into(),
        })
    }
    /// Validated native schema identifier. No quoting is added.
    pub fn schema(&self) -> &str {
        &self.schema
    }
    /// Validated native table identifier. No quoting is added.
    pub fn table(&self) -> &str {
        &self.table
    }
}
impl std::fmt::Debug for ControlTableName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ControlTableName { redacted }")
    }
}
/// Shared positive whole-operation bounds. Native statement and lock rules stay in each driver.
#[derive(Clone, Copy, Debug)]
pub struct OperationDeadlines {
    connect: Duration,
    io: Duration,
}
impl OperationDeadlines {
    /// Admit positive connect and whole-I/O durations at most60seconds.
    pub fn new(connect: Duration, io: Duration) -> Result<Self, OwnerError> {
        let max = Duration::from_secs(60);
        if connect.is_zero() || io.is_zero() || connect > max || io > max {
            return Err(OwnerError::Invalid);
        }
        Ok(Self { connect, io })
    }
    /// Complete connection establishment limit, including handshake and authentication.
    pub fn connect(self) -> Duration {
        self.connect
    }
    /// Complete native operation limit, including response consumption.
    pub fn io(self) -> Duration {
        self.io
    }
}
