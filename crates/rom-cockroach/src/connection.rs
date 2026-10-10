use crate::{ControlTable, Deadlines, Transaction};
use rom_sql_core::{OperationDeadlines, OwnerError};
use tokio_postgres::{Config, Row, Socket, tls::MakeTlsConnect};
/// One privately owned PGwire client. Uncertainty permanently retires its transport.
/// This blocking facade must run outside an active Tokio runtime.
pub struct Connection {
    wire: rom_pgwire::Connection,
}
impl Connection {
    /// Connect through the host's TLS connector. Require encryption, one explicit endpoint,
    /// and no hostaddr override. Certificate identity and trust remain the host's responsibility.
    pub fn connect<T>(config: Config, tls: T, deadlines: Deadlines) -> Result<Self, OwnerError>
    where
        T: MakeTlsConnect<Socket>,
        T::Stream: Send + 'static,
    {
        let bounds = OperationDeadlines::new(deadlines.connect, deadlines.io)?;
        let wire = rom_pgwire::Connection::connect(config, tls, bounds).map_err(map_error)?;
        Self::admit(wire, deadlines)
    }
    /// Explicit plaintext fixture mode, admitted only for one literal loopback TCP IP.
    /// No other constructor downgrades TLS or supplies a default endpoint.
    pub fn connect_loopback(config: Config, deadlines: Deadlines) -> Result<Self, OwnerError> {
        let bounds = OperationDeadlines::new(deadlines.connect, deadlines.io)?;
        let wire = rom_pgwire::Connection::connect_loopback(config, bounds).map_err(map_error)?;
        Self::admit(wire, deadlines)
    }
    fn admit(mut wire: rom_pgwire::Connection, deadlines: Deadlines) -> Result<Self, OwnerError> {
        let sql = format!(
            "SET enable_durable_locking_for_serializable=true; SET optimizer_use_lock_op_for_serializable=true; SET statement_timeout='{}ms'; SET lock_timeout='{}ms'; SET idle_in_transaction_session_timeout='{}ms'",
            deadlines.statement_ms, deadlines.lock_ms, deadlines.statement_ms
        );
        wire.batch(&sql).map_err(map_error)?;
        let limits = rom_pgwire::MetadataLimits::new(1, 6, 1024).map_err(map_error)?;
        let rows = wire.query_metadata(
            "SELECT version(),current_setting('enable_durable_locking_for_serializable'),current_setting('optimizer_use_lock_op_for_serializable'),current_setting('statement_timeout'),current_setting('lock_timeout'),current_setting('idle_in_transaction_session_timeout')",
            &[], limits
        ).map_err(map_error)?;
        if rows.len() != 1 || rows[0].len() != 6 {
            return Err(OwnerError::Invalid);
        }
        let row = &rows[0];
        let text = |index| {
            row.try_get::<_, &str>(index)
                .map_err(|_| OwnerError::Invalid)
        };
        if !text(0)?.starts_with("CockroachDB CCL v26.3.")
            || text(1)? != "on"
            || text(2)? != "on"
            || text(3)? != deadlines.statement_ms.to_string()
            || text(4)? != deadlines.lock_ms.to_string()
            || text(5)? != deadlines.statement_ms.to_string()
        {
            return Err(OwnerError::Unavailable);
        }
        Ok(Self { wire })
    }
    /// Whether native use remains possible; this does not establish liveness or ownership.
    pub fn is_available(&self) -> bool {
        self.wire.is_available()
    }
    /// Open SERIALIZABLE after verified native durable-lock and bounded session admission.
    /// Table provisioning and ownership arbitration remain separate.
    pub fn begin<'a>(&'a mut self, table: &'a ControlTable) -> Result<Transaction<'a>, OwnerError> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(OwnerError::Invalid);
        }
        if !self.is_available() {
            return Err(OwnerError::Unavailable);
        }
        let sql = "BEGIN ISOLATION LEVEL SERIALIZABLE";
        if self.batch(sql).is_err() {
            self.retire();
            return Err(OwnerError::Unknown);
        }
        Ok(Transaction::new(self, table))
    }
    pub(crate) fn batch(&mut self, sql: &str) -> Result<(), OwnerError> {
        let result = self.wire.batch(sql);
        self.classify(result)
    }
    pub(crate) fn execute(
        &mut self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<u64, OwnerError> {
        let result = self.wire.execute(sql, params);
        self.classify(result)
    }
    pub(crate) fn query(
        &mut self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<Row>, OwnerError> {
        // Two complete projected rows: INT4 +32-byte identity +INT8 +32-byte token +INT8 length.
        let limits = rom_pgwire::MetadataLimits::new(2, 5, 168).map_err(map_error)?;
        let result = self.wire.query_metadata(sql, params, limits);
        self.classify(result)
    }
    fn classify<R>(&mut self, result: Result<R, rom_pgwire::Error>) -> Result<R, OwnerError> {
        if result.as_ref().err().and_then(rom_pgwire::Error::sqlstate) == Some("40003") {
            self.retire();
            return Err(OwnerError::Unknown);
        }
        result.map_err(map_error)
    }
    pub(crate) fn cleanup(&mut self) -> Result<(), OwnerError> {
        // Complete ROLLBACK/ReadyForQuery precedes any admitted subsequent request.
        if self.batch("ROLLBACK").is_ok() {
            Ok(())
        } else {
            self.retire();
            Err(OwnerError::Unknown)
        }
    }
    pub(crate) fn retire(&mut self) {
        self.wire.retire();
    }
}
fn map_error(error: rom_pgwire::Error) -> OwnerError {
    match error {
        rom_pgwire::Error::Invalid | rom_pgwire::Error::ResponseLimit => OwnerError::Invalid,
        rom_pgwire::Error::Unavailable => OwnerError::Unavailable,
        rom_pgwire::Error::Unknown => OwnerError::Unknown,
        rom_pgwire::Error::Server(code)
            if code == tokio_postgres::error::SqlState::UNDEFINED_TABLE =>
        {
            OwnerError::Uninitialized
        }
        rom_pgwire::Error::Server(_) => OwnerError::Unavailable,
    }
}
impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Connection { redacted }")
    }
}
