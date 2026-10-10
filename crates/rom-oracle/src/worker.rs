use crate::{Config, ControlTable, Deadlines, TlsWallet, Value, metadata};
use rom_sql_core::{OwnerError, OwnerState};
pub(crate) enum Failure {
    Native(oracle::Error),
    Rejected(OwnerError),
}
impl From<oracle::Error> for Failure {
    fn from(e: oracle::Error) -> Self {
        Self::Native(e)
    }
}
pub(crate) struct Worker {
    client: Option<oracle::Connection>,
}
impl Worker {
    pub(crate) fn open(
        config: Config,
        tls: Option<TlsWallet>,
        bounds: Deadlines,
    ) -> Result<Self, OwnerError> {
        let descriptor = config.descriptor(bounds, tls.as_ref());
        let mut c = oracle::Connection::connect(config.user, config.password, descriptor)
            .map_err(|_| OwnerError::Unavailable)?;
        c.set_autocommit(false);
        c.set_call_timeout(Some(bounds.roundtrip))
            .map_err(|_| OwnerError::Unavailable)?;
        c.set_stmt_cache_size(16)
            .map_err(|_| OwnerError::Unavailable)?;
        if c.autocommit()
            || c.call_timeout().map_err(|_| OwnerError::Unavailable)? != Some(bounds.roundtrip)
            || c.server_version()
                .map_err(|_| OwnerError::Unavailable)?
                .0
                .major()
                != 23
        {
            return Err(OwnerError::Unavailable);
        }
        Ok(Self { client: Some(c) })
    }
    fn run<R>(
        &mut self,
        operation: impl FnOnce(&oracle::Connection) -> Result<R, Failure>,
    ) -> Result<R, OwnerError> {
        let result = operation(self.client.as_ref().ok_or(OwnerError::Unavailable)?);
        match result {
            Ok(v) => Ok(v),
            Err(Failure::Rejected(e)) => Err(e),
            Err(Failure::Native(e)) => match e.oci_code() {
                Some(942) => Err(OwnerError::Uninitialized),
                Some(1 | 54 | 60 | 1400 | 1438 | 2290 | 2291 | 2292 | 30006) => {
                    Err(OwnerError::Unavailable)
                }
                _ => {
                    self.retire();
                    Err(OwnerError::Unknown)
                }
            },
        }
    }
    pub(crate) fn begin(&mut self) -> Result<(), OwnerError> {
        self.terminal(false)?;
        self.run(|c| {
            c.execute("SET TRANSACTION ISOLATION LEVEL READ COMMITTED", &[])?;
            Ok(())
        })
    }
    pub(crate) fn read_owner(
        &mut self,
        table: ControlTable,
        lock: u64,
    ) -> Result<OwnerState, OwnerError> {
        self.run(|c| metadata::read(c, &table, lock))
    }
    pub(crate) fn execute(&mut self, sql: String, values: Vec<Value>) -> Result<u64, OwnerError> {
        self.run(|c| {
            let mut stmt = c.statement(&sql).build()?;
            if !stmt.is_dml() || stmt.is_returning() {
                return Err(Failure::Rejected(OwnerError::Invalid));
            }
            let params: Vec<_> = values.iter().map(Value::native).collect();
            stmt.execute(&params)?;
            Ok(stmt.row_count()?)
        })
    }
    pub(crate) fn terminal(&mut self, commit: bool) -> Result<(), OwnerError> {
        let c = self.client.as_ref().ok_or(OwnerError::Unavailable)?;
        let result = if commit { c.commit() } else { c.rollback() };
        if result.is_err() {
            self.retire();
            return Err(OwnerError::Unknown);
        }
        Ok(())
    }
    pub(crate) fn retire(&mut self) {
        if let Some(c) = self.client.take() {
            let _ = c.close();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.retire();
    }
}
