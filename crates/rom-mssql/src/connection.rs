use crate::{ControlTable, Deadlines, Transaction};
use rom_sql_core::OwnerError;
use std::ops::AsyncFnOnce;
use tiberius::{Client, Config, EncryptionLevel, Row};
use tokio::{
    net::TcpStream,
    runtime::{Builder, Runtime},
    time::timeout,
};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
pub(crate) type NativeClient = Client<Compat<TcpStream>>;
/// One worker-owned encrypted native connection with permanent retirement on transport uncertainty.
/// This blocking facade must run outside an active Tokio runtime, normally on a SQL executor worker.
/// Native SDK tracing is controlled by the host; do not enable SQL/credential tracing sinks.
pub struct Connection {
    runtime: Option<Runtime>,
    pub(crate) client: Option<NativeClient>,
    deadlines: Deadlines,
}
impl Connection {
    /// Connect to the explicit host configuration. Force encryption; retain its certificate policy.
    /// No retries, alternate endpoints, certificate bypass or automatic table creation are added.
    pub fn connect(mut config: Config, deadlines: Deadlines) -> Result<Self, OwnerError> {
        outside_runtime()?;
        config.encryption(EncryptionLevel::Required);
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| OwnerError::Unavailable)?;
        let client = runtime
            .block_on(async {
                timeout(deadlines.connect, async {
                    let tcp = TcpStream::connect(config.get_addr()).await?;
                    tcp.set_nodelay(true)?;
                    Client::connect(config, tcp.compat_write()).await
                })
                .await
            })
            .map_err(|_| OwnerError::Unavailable)?
            .map_err(|_| OwnerError::Unavailable)?;
        Ok(Self {
            runtime: Some(runtime),
            client: Some(client),
            deadlines,
        })
    }
    /// Whether native use is still possible. This is not a liveness or ownership check.
    pub fn is_available(&self) -> bool {
        self.client.is_some()
    }
    /// Open READ COMMITTED on a clean connection. Validate the row only during core arbitration.
    pub fn begin<'a>(&'a mut self, table: &'a ControlTable) -> Result<Transaction<'a>, OwnerError> {
        outside_runtime()?;
        let lock_ms = self.deadlines.lock_ms;
        let setup = format!(
            "SET XACT_ABORT ON; SET LOCK_TIMEOUT {lock_ms}; SELECT XACT_STATE(), @@TRANCOUNT"
        );
        // Session settings must use SQLBatch; sp_executesql scopes them to its RPC.
        let rows = self.run(async |c| c.simple_query(&setup).await?.into_results().await)?;
        if !clean(&rows) {
            self.retire();
            return Err(OwnerError::Unknown);
        }
        // Any unacknowledged BEGIN must retire its possibly active connection.
        if self
            .run(async |c| {
                c.begin_transaction_with_isolation(tiberius::IsolationLevel::ReadCommitted)
                    .await
            })
            .is_err()
        {
            self.retire();
            return Err(OwnerError::Unknown);
        }
        Ok(Transaction::new(self, table))
    }
    pub(crate) fn retire(&mut self) {
        self.client.take();
    }
    pub(crate) fn run<R>(
        &mut self,
        op: impl AsyncFnOnce(&mut NativeClient) -> tiberius::Result<R>,
    ) -> Result<R, OwnerError> {
        outside_runtime()?;
        let client = self.client.as_mut().ok_or(OwnerError::Unavailable)?;
        let result = self
            .runtime
            .as_ref()
            .ok_or(OwnerError::Unavailable)?
            .block_on(async { timeout(self.deadlines.io, op(client)).await });
        match result {
            Ok(Ok(value)) => Ok(value),
            // Tiberius reports statement errors at end-of-message after processing ENVCHANGE.
            // Whole transaction rollback is still required; never report success from this branch.
            Ok(Err(tiberius::error::Error::Server(_))) => Err(OwnerError::Unavailable),
            _ => {
                self.retire();
                Err(OwnerError::Unknown)
            }
        }
    }
    pub(crate) fn query(
        &mut self,
        sql: &str,
        params: &[&dyn tiberius::ToSql],
    ) -> Result<Vec<Vec<Row>>, OwnerError> {
        self.run(async |c| c.query(sql, params).await?.into_results().await)
    }
    pub(crate) fn cleanup(&mut self) -> Result<(), OwnerError> {
        let result = self.run(async |c| {
            c.simple_query(
                "IF XACT_STATE() <> 0 ROLLBACK TRANSACTION; SELECT XACT_STATE(), @@TRANCOUNT",
            )
            .await?
            .into_results()
            .await
        });
        if result.as_ref().is_ok_and(|rows| clean(rows)) {
            Ok(())
        } else {
            self.retire();
            Err(OwnerError::Unknown)
        }
    }
}
fn clean(rows: &[Vec<Row>]) -> bool {
    rows.len() == 1
        && rows[0].len() == 1
        && rows[0][0].len() == 2
        && rows[0][0].try_get::<i32, _>(0).ok().flatten() == Some(0)
        && rows[0][0].try_get::<i32, _>(1).ok().flatten() == Some(0)
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
        // No adapter blocking tasks exist. This also avoids Tokio's runtime-Drop panic
        // when a host moves a retired handle back into an async context.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}
