use crate::{Config, Deadlines, Profile, SslOpts, Value};
use mysql_async::{Conn, OptsBuilder, Params, Row, prelude::Queryable};
use rom_sql_core::OwnerError;
use std::{ops::AsyncFnOnce, time::Duration};
use tokio::{
    runtime::{Builder, Runtime},
    time::timeout,
};
pub(crate) enum Failure {
    Native(mysql_async::Error),
    Protocol,
}
impl From<mysql_async::Error> for Failure {
    fn from(e: mysql_async::Error) -> Self {
        Self::Native(e)
    }
}
pub(crate) struct Worker {
    runtime: Option<Runtime>,
    client: Option<Conn>,
    io: Duration,
}
impl Worker {
    pub(crate) fn open(
        config: Config,
        tls: Option<SslOpts>,
        deadlines: Deadlines,
    ) -> Result<Self, OwnerError> {
        let opts = OptsBuilder::default()
            .ip_or_hostname(config.host)
            .tcp_port(config.port)
            .user(Some(config.user))
            .pass(Some(config.password))
            .db_name(Some(config.database))
            .prefer_socket(false)
            .ssl_opts(tls)
            .max_allowed_packet(Some(65536))
            .stmt_cache_size(Some(32))
            .client_found_rows(true)
            .enable_cleartext_plugin(false);
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| OwnerError::Unavailable)?;
        let result = runtime.block_on(async { timeout(deadlines.connect, Conn::new(opts)).await });
        let client = match result {
            Ok(Ok(c)) => c,
            _ => {
                runtime.shutdown_background();
                return Err(OwnerError::Unavailable);
            }
        };
        let mut worker = Self {
            runtime: Some(runtime),
            client: Some(client),
            io: deadlines.io,
        };
        let rows = worker.query(
            "SELECT SUBSTRING(VERSION(),1,64),@@innodb_flush_log_at_trx_commit".into(),
            vec![],
            2,
            1,
        )?;
        let row = rows.first().ok_or(OwnerError::Unavailable)?;
        let version = row
            .get_opt::<String, _>(0)
            .ok_or(OwnerError::Unavailable)?
            .map_err(|_| OwnerError::Unavailable)?;
        let durable = row
            .get_opt::<u64, _>(1)
            .ok_or(OwnerError::Unavailable)?
            .map_err(|_| OwnerError::Unavailable)?;
        if durable != 1
            || match config.profile {
                Profile::MySql => version.contains("MariaDB") || !version.starts_with("8."),
                Profile::MariaDb => {
                    !version.contains("MariaDB")
                        || !(version.starts_with("10.") || version.starts_with("11."))
                }
            }
        {
            return Err(OwnerError::Unavailable);
        }
        Ok(worker)
    }
    fn run<R>(
        &mut self,
        op: impl AsyncFnOnce(&mut Conn) -> Result<R, Failure>,
    ) -> Result<R, OwnerError> {
        let c = self.client.as_mut().ok_or(OwnerError::Unavailable)?;
        let result = self
            .runtime
            .as_ref()
            .ok_or(OwnerError::Unavailable)?
            .block_on(async { timeout(self.io, op(c)).await });
        match result {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(Failure::Native(mysql_async::Error::Server(e)))) => {
                if e.code == 1146 {
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
    pub(crate) fn begin(&mut self, lock: u64) -> Result<(), OwnerError> {
        let result = self.run(async |c| {
            c.query_drop(format!("SET SESSION innodb_lock_wait_timeout={lock}"))
                .await?;
            c.query_drop("SET SESSION TRANSACTION ISOLATION LEVEL READ COMMITTED")
                .await?;
            c.query_drop("START TRANSACTION").await?;
            Ok(())
        });
        if result.is_err() {
            self.retire();
            return Err(OwnerError::Unknown);
        }
        Ok(())
    }
    pub(crate) fn query(
        &mut self,
        sql: String,
        params: Vec<Value>,
        columns: usize,
        max_rows: usize,
    ) -> Result<Vec<Row>, OwnerError> {
        self.run(async |c| {
            let mut result = c.exec_iter(sql, Params::Positional(params)).await?;
            if result.columns_ref().len() != columns {
                return Err(Failure::Protocol);
            }
            let mut rows = Vec::new();
            while let Some(row) = result.next().await? {
                if rows.len() == max_rows || row.len() != columns {
                    return Err(Failure::Protocol);
                }
                rows.push(row);
            }
            if !result.is_empty() {
                return Err(Failure::Protocol);
            }
            result.drop_result().await?;
            Ok(rows)
        })
    }
    pub(crate) fn execute(&mut self, sql: String, params: Vec<Value>) -> Result<u64, OwnerError> {
        self.run(async |c| {
            let result = c.exec_iter(sql, Params::Positional(params)).await?;
            if !result.columns_ref().is_empty() || !result.is_empty() {
                return Err(Failure::Protocol);
            }
            let affected = result.affected_rows();
            result.drop_result().await?;
            Ok(affected)
        })
    }
    pub(crate) fn terminal(&mut self, commit: bool) -> Result<(), OwnerError> {
        let result = self.run(async |c| {
            c.query_drop(if commit { "COMMIT" } else { "ROLLBACK" })
                .await?;
            Ok(())
        });
        if result.is_err() {
            self.retire();
            return Err(OwnerError::Unknown);
        }
        Ok(())
    }
    pub(crate) fn retire(&mut self) {
        // Executor invokes this outside block_on: SDK Drop cannot spawn Tokio cleanup.
        self.client.take();
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background()
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.retire()
    }
}
