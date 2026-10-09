use crate::{ControlTable, Deadlines, Transaction};
use rom_sql_core::OwnerError;
use std::ops::AsyncFnOnce;
use tokio::{
    runtime::{Builder, Runtime},
    task::JoinHandle,
    time::timeout,
};
use tokio_postgres::{
    Client, Config, Row, Socket,
    config::{Host, SslMode},
    tls::MakeTlsConnect,
};
/// One worker-owned client and transport task. Uncertainty permanently retires both.
/// This blocking facade must run outside an active Tokio runtime.
pub struct Connection {
    runtime: Option<Runtime>,
    client: Option<Client>,
    transport: Option<JoinHandle<Result<(), tokio_postgres::Error>>>,
    deadlines: Deadlines,
}
impl Connection {
    /// Connect through the host's TLS connector. Require encryption, one explicit endpoint,
    /// and no hostaddr override. Certificate identity and trust remain the host's responsibility.
    pub fn connect<T>(mut config: Config, tls: T, deadlines: Deadlines) -> Result<Self, OwnerError>
    where
        T: MakeTlsConnect<Socket>,
        T::Stream: Send + 'static,
    {
        endpoint(&config)?;
        config.ssl_mode(SslMode::Require);
        Self::open(config, tls, deadlines)
    }
    /// Explicit plaintext fixture mode, admitted only for one literal loopback TCP IP.
    /// No other constructor downgrades TLS or supplies a default endpoint.
    pub fn connect_loopback(mut config: Config, deadlines: Deadlines) -> Result<Self, OwnerError> {
        endpoint(&config)?;
        if !matches!(&config.get_hosts()[0], Host::Tcp(host) if host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback()))
        {
            return Err(OwnerError::Invalid);
        }
        config.ssl_mode(SslMode::Disable);
        Self::open(config, tokio_postgres::NoTls, deadlines)
    }
    fn open<T>(mut config: Config, tls: T, deadlines: Deadlines) -> Result<Self, OwnerError>
    where
        T: MakeTlsConnect<Socket>,
        T::Stream: Send + 'static,
    {
        outside_runtime()?;
        config.connect_timeout(deadlines.connect);
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| OwnerError::Unavailable)?;
        let (client, transport) = runtime.block_on(async {
            let (client, connection) = timeout(deadlines.connect, config.connect(tls))
                .await
                .map_err(|_| OwnerError::Unavailable)?
                .map_err(|_| OwnerError::Unavailable)?;
            Ok::<_, OwnerError>((client, tokio::spawn(connection)))
        })?;
        Ok(Self {
            runtime: Some(runtime),
            client: Some(client),
            transport: Some(transport),
            deadlines,
        })
    }
    /// Whether native use remains possible; this does not establish liveness or ownership.
    pub fn is_available(&self) -> bool {
        self.client.is_some()
    }
    /// Open READ COMMITTED with native statement/lock bounds and synchronous commit.
    /// Table provisioning and ownership arbitration remain separate.
    pub fn begin<'a>(&'a mut self, table: &'a ControlTable) -> Result<Transaction<'a>, OwnerError> {
        outside_runtime()?;
        if self.client.is_none() {
            return Err(OwnerError::Unavailable);
        }
        if self
            .transport
            .as_ref()
            .is_none_or(|task| task.is_finished())
        {
            self.retire();
            return Err(OwnerError::Unavailable);
        }
        let sql = format!(
            "BEGIN ISOLATION LEVEL READ COMMITTED; SET LOCAL statement_timeout={}; SET LOCAL lock_timeout={}; SET LOCAL idle_in_transaction_session_timeout={}; SET LOCAL synchronous_commit=on",
            self.deadlines.statement_ms, self.deadlines.lock_ms, self.deadlines.statement_ms
        );
        if self
            .run(async |client| client.batch_execute(&sql).await)
            .is_err()
        {
            self.retire();
            return Err(OwnerError::Unknown);
        }
        Ok(Transaction::new(self, table))
    }
    pub(crate) fn run<R>(
        &mut self,
        op: impl AsyncFnOnce(&Client) -> Result<R, tokio_postgres::Error>,
    ) -> Result<R, OwnerError> {
        outside_runtime()?;
        let client = self.client.as_ref().ok_or(OwnerError::Unavailable)?;
        let result = self
            .runtime
            .as_ref()
            .ok_or(OwnerError::Unavailable)?
            .block_on(async { timeout(self.deadlines.io, op(client)).await });
        match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) if error.code().is_some() => {
                if error.code() == Some(&tokio_postgres::error::SqlState::UNDEFINED_TABLE) {
                    Err(OwnerError::Uninitialized)
                } else {
                    Err(OwnerError::Unavailable)
                }
            }
            _ => {
                self.retire();
                Err(OwnerError::Unknown)
            }
        }
    }
    pub(crate) fn query(
        &mut self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<Row>, OwnerError> {
        self.run(async |c| c.query(sql, params).await)
    }
    pub(crate) fn cleanup(&mut self) -> Result<(), OwnerError> {
        // batch_execute consumes the complete terminal response. A later admitted request
        // cannot run until PostgreSQL has acknowledged ROLLBACK/ReadyForQuery.
        if self
            .run(async |c| c.batch_execute("ROLLBACK").await)
            .is_ok()
        {
            Ok(())
        } else {
            self.retire();
            Err(OwnerError::Unknown)
        }
    }
    pub(crate) fn retire(&mut self) {
        self.client.take();
        if let Some(task) = self.transport.take() {
            task.abort();
            if tokio::runtime::Handle::try_current().is_err()
                && let Some(runtime) = &self.runtime
            {
                // Await task destruction so its native socket closes before recovery.
                runtime.block_on(async {
                    let _ = task.await;
                });
            }
        }
    }
}
fn endpoint(config: &Config) -> Result<(), OwnerError> {
    if !matches!(config.get_hosts().first(),Some(Host::Tcp(host)) if !host.is_empty())
        || config.get_hosts().len() != 1
        || !config.get_hostaddrs().is_empty()
        || config.get_ports().len() > 1
    {
        return Err(OwnerError::Invalid);
    }
    Ok(())
}
fn outside_runtime() -> Result<(), OwnerError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        Err(OwnerError::Invalid)
    } else {
        Ok(())
    }
}
impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Connection { redacted }")
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.retire();
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}
