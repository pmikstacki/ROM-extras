use crate::{Config, Error, MetadataLimits, Row, Socket, ToSql, metadata::FieldBytes};
use futures_core::Stream;
use rom_sql_core::OperationDeadlines;
use std::{future::poll_fn, ops::AsyncFnOnce};
use tokio::{
    runtime::{Builder, Runtime},
    task::JoinHandle,
    time::timeout,
};
use tokio_postgres::{
    Client,
    config::{Host, SslMode},
    tls::MakeTlsConnect,
};

/// A private runtime, native client and retained transport task with complete I/O bounds.
/// Provider transaction policy stays outside this blocking protocol boundary.
pub struct Connection {
    runtime: Option<Runtime>,
    client: Option<Client>,
    transport: Option<JoinHandle<Result<(), tokio_postgres::Error>>>,
    deadlines: OperationDeadlines,
}
impl Connection {
    /// Require encrypted TCP to one explicit host. The host connector verifies identity/trust.
    pub fn connect<T>(
        mut config: Config,
        tls: T,
        deadlines: OperationDeadlines,
    ) -> Result<Self, Error>
    where
        T: MakeTlsConnect<Socket>,
        T::Stream: Send + 'static,
    {
        endpoint(&config)?;
        config.ssl_mode(SslMode::Require);
        Self::open(config, tls, deadlines)
    }
    /// Explicit plaintext fixture access to one literal loopback TCP IP only.
    pub fn connect_loopback(
        mut config: Config,
        deadlines: OperationDeadlines,
    ) -> Result<Self, Error> {
        endpoint(&config)?;
        if !matches!(&config.get_hosts()[0], Host::Tcp(host) if host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback()))
        {
            return Err(Error::Invalid);
        }
        config.ssl_mode(SslMode::Disable);
        Self::open(config, tokio_postgres::NoTls, deadlines)
    }
    fn open<T>(mut config: Config, tls: T, deadlines: OperationDeadlines) -> Result<Self, Error>
    where
        T: MakeTlsConnect<Socket>,
        T::Stream: Send + 'static,
    {
        outside_runtime()?;
        config.connect_timeout(deadlines.connect());
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Error::Unavailable)?;
        let (client, transport) = runtime.block_on(async {
            let (client, connection) = timeout(deadlines.connect(), config.connect(tls))
                .await
                .map_err(|_| Error::Unavailable)?
                .map_err(|_| Error::Unavailable)?;
            Ok::<_, Error>((client, tokio::spawn(connection)))
        })?;
        Ok(Self {
            runtime: Some(runtime),
            client: Some(client),
            transport: Some(transport),
            deadlines,
        })
    }
    /// Whether native use remains possible; this establishes neither liveness nor ownership.
    pub fn is_available(&self) -> bool {
        self.client.is_some()
    }

    /// Complete bounded provider session/terminal SQL, without arbitrary host callbacks.
    pub fn batch(&mut self, sql: &str) -> Result<(), Error> {
        request(sql, 0)?;
        self.run(async |client| client.batch_execute(sql).await.map_err(native_error))
    }
    /// Complete trusted prepared persistence; providers control transaction and ownership policy.
    /// SQL is capped at16KiB/64parameters. Hosts must bound parameter payloads and total work.
    pub fn execute(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        request(sql, params.len())?;
        self.run(async |client| {
            let stream = client
                .query_raw(sql, params.iter().copied())
                .await
                .map_err(native_error)?;
            let mut stream = std::pin::pin!(stream);
            if let Some(row) = poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
                row.map_err(native_error)?;
                return Err(Error::ResponseLimit);
            }
            stream.rows_affected().ok_or(Error::Unknown)
        })
    }
    /// Consume a complete metadata stream within fixed retained row/column/decoded-byte bounds.
    /// Providers must use fixed, bounded SQL projections; this is not an application query API.
    pub fn query_metadata(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        limits: MetadataLimits,
    ) -> Result<Vec<Row>, Error> {
        request(sql, params.len())?;
        self.run(async |client| {
            let stream = client
                .query_raw(sql, params.iter().copied())
                .await
                .map_err(native_error)?;
            let mut stream = std::pin::pin!(stream);
            let mut rows = Vec::new();
            let mut bytes = 0usize;
            while let Some(row) = poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
                let row = row.map_err(native_error)?;
                if rows.len() >= limits.rows || row.len() > limits.columns {
                    return Err(Error::ResponseLimit);
                }
                for index in 0..row.len() {
                    let value = row
                        .try_get::<_, Option<FieldBytes<'_>>>(index)
                        .map_err(|_| Error::ResponseLimit)?;
                    bytes = bytes
                        .checked_add(value.map_or(0, |value| value.0.len()))
                        .ok_or(Error::ResponseLimit)?;
                    if bytes > limits.bytes {
                        return Err(Error::ResponseLimit);
                    }
                }
                rows.push(row);
            }
            Ok(rows)
        })
    }
    fn run<R>(&mut self, op: impl AsyncFnOnce(&Client) -> Result<R, Error>) -> Result<R, Error> {
        outside_runtime()?;
        if self
            .transport
            .as_ref()
            .is_none_or(|task| task.is_finished())
        {
            self.retire();
            return Err(Error::Unavailable);
        }
        let client = self.client.as_ref().ok_or(Error::Unavailable)?;
        let result = self
            .runtime
            .as_ref()
            .ok_or(Error::Unavailable)?
            .block_on(async { timeout(self.deadlines.io(), op(client)).await });
        match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error @ Error::Server(_))) => Err(error),
            Ok(Err(error @ Error::ResponseLimit)) => {
                self.retire();
                Err(error)
            }
            _ => {
                self.retire();
                Err(Error::Unknown)
            }
        }
    }
    /// Permanently retire the native transport; await owned task destruction outside host Tokio.
    pub fn retire(&mut self) {
        self.client.take();
        if let Some(task) = self.transport.take() {
            task.abort();
            if tokio::runtime::Handle::try_current().is_err()
                && let Some(runtime) = &self.runtime
            {
                runtime.block_on(async {
                    let _ = task.await;
                });
            }
        }
    }
}
fn request(sql: &str, parameters: usize) -> Result<(), Error> {
    if sql.is_empty() || sql.len() > 16384 || parameters > 64 {
        Err(Error::Invalid)
    } else {
        Ok(())
    }
}
fn native_error(error: tokio_postgres::Error) -> Error {
    match error.code() {
        Some(code)
            if code.code().len() == 5
                && code
                    .code()
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit()) =>
        {
            Error::Server(code.clone())
        }
        _ => Error::Unknown,
    }
}
fn endpoint(config: &Config) -> Result<(), Error> {
    if !matches!(config.get_hosts().first(), Some(Host::Tcp(host)) if !host.is_empty())
        || config.get_hosts().len() != 1
        || !config.get_hostaddrs().is_empty()
        || config.get_ports().len() > 1
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn outside_runtime() -> Result<(), Error> {
    if tokio::runtime::Handle::try_current().is_ok() {
        Err(Error::Invalid)
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
