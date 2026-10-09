use rom_sql_core::{ControlTableName, OperationDeadlines, OwnerError};
use std::time::Duration;
/// Explicit vendor; the same protocol driver receives separate native qualification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// MySQL8 family; currently qualified profile is recorded separately.
    MySql,
    /// MariaDB10/11 families; currently qualified profile is recorded separately.
    MariaDb,
}
/// Minimal host-owned endpoint and credentials, without SDK callbacks or implicit endpoints.
pub struct Config {
    pub(crate) profile: Profile,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) password: String,
    pub(crate) database: String,
}
impl Config {
    /// Construct an explicit endpoint. Secrets remain on the host and are never Debug output.
    pub fn new(
        profile: Profile,
        host: &str,
        port: u16,
        user: &str,
        password: String,
        database: &str,
    ) -> Result<Self, OwnerError> {
        if host.is_empty()
            || host.len() > 253
            || !host.is_ascii()
            || host.contains(['\0', '/', '?', '#', '@'])
            || host.chars().any(char::is_whitespace)
            || port == 0
            || user.is_empty()
            || user.len() > 64
            || user.contains('\0')
            || password.len() > 4096
            || password.contains('\0')
        {
            return Err(OwnerError::Invalid);
        }
        ControlTableName::new(database, "control")?;
        Ok(Self {
            profile,
            host: host.into(),
            port,
            user: user.into(),
            password,
            database: database.into(),
        })
    }
}
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Config { redacted }")
    }
}
/// Whole connect/I/O limits and native whole-second InnoDB lock timeout.
#[derive(Clone, Copy, Debug)]
pub struct Deadlines {
    pub(crate) connect: Duration,
    pub(crate) io: Duration,
    pub(crate) lock_seconds: u64,
}
impl Deadlines {
    /// Require positive connect/I/O up to60s and whole-second 0<lock<io.
    pub fn new(connect: Duration, io: Duration, lock: Duration) -> Result<Self, OwnerError> {
        let common = OperationDeadlines::new(connect, io)?;
        if lock.is_zero() || lock.subsec_nanos() != 0 || lock >= io {
            return Err(OwnerError::Invalid);
        }
        Ok(Self {
            connect: common.connect(),
            io: common.io(),
            lock_seconds: lock.as_secs(),
        })
    }
}
/// Immutable validated provisioned singleton location. Provisioning is a separate host task.
#[derive(Clone)]
pub struct ControlTable {
    pub(crate) names: ControlTableName,
    pub(crate) qualified: String,
    pub(crate) identity: [u8; 32],
}
impl ControlTable {
    /// Validate portable names and preserve exact store identity; no table is created or repaired.
    pub fn new(database: &str, table: &str, identity: [u8; 32]) -> Result<Self, OwnerError> {
        let names = ControlTableName::new(database, table)?;
        Ok(Self {
            qualified: format!("`{}`.`{}`", names.schema(), names.table()),
            names,
            identity,
        })
    }
}
impl std::fmt::Debug for ControlTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ControlTable { redacted }")
    }
}
