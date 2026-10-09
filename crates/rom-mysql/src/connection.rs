use crate::{Config, ControlTable, Deadlines, SslOpts, Transaction, worker::Worker};
use rom_sql_core::{Executor, ExecutorError, OwnerError};
/// Private generic executor owns one unpooled native client and runtime.
/// Blocking calls refuse active Tokio callers; use a host-owned bounded blocking bridge.
pub struct Connection {
    executor: Executor<Worker>,
    deadlines: Deadlines,
    available: bool,
}
impl Connection {
    /// Require host-approved TLS trust without verification bypass or hostname override.
    pub fn connect(config: Config, tls: SslOpts, deadlines: Deadlines) -> Result<Self, OwnerError> {
        if tls.accept_invalid_certs()
            || tls.skip_domain_validation()
            || tls.tls_hostname_override().is_some()
        {
            return Err(OwnerError::Invalid);
        }
        Self::open(config, Some(tls), deadlines)
    }
    /// Explicit plaintext native fixture mode; require one literal loopback TCP IP.
    pub fn connect_loopback(config: Config, deadlines: Deadlines) -> Result<Self, OwnerError> {
        if !config
            .host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
        {
            return Err(OwnerError::Invalid);
        }
        Self::open(config, None, deadlines)
    }
    fn open(
        config: Config,
        tls: Option<SslOpts>,
        deadlines: Deadlines,
    ) -> Result<Self, OwnerError> {
        outside_runtime()?;
        let executor = Executor::spawn(1, move || {
            Worker::open(config, tls, deadlines).map_err(|_| ExecutorError::Initialization)
        })
        .map_err(|_| OwnerError::Unavailable)?;
        Ok(Self {
            executor,
            deadlines,
            available: true,
        })
    }
    /// Native use remains admitted; this does not prove liveness or current ownership.
    pub fn is_available(&self) -> bool {
        self.available
    }
    /// Start READ COMMITTED with a bounded native lock wait; no provisioning or arbitration.
    pub fn begin<'a>(&'a mut self, table: &'a ControlTable) -> Result<Transaction<'a>, OwnerError> {
        let lock = self.deadlines.lock_seconds;
        self.call(move |worker| worker.begin(lock))?;
        Ok(Transaction::new(self, table))
    }
    pub(crate) fn call<R: Send + 'static>(
        &mut self,
        operation: impl FnOnce(&mut Worker) -> Result<R, OwnerError> + Send + 'static,
    ) -> Result<R, OwnerError> {
        outside_runtime()?;
        if !self.available {
            return Err(OwnerError::Unavailable);
        }
        let result = self
            .executor
            .execute(operation)
            .unwrap_or(Err(OwnerError::Unknown));
        if matches!(&result, Err(OwnerError::Unknown)) {
            self.available = false
        }
        result
    }
    pub(crate) fn cleanup(&mut self) -> Result<(), OwnerError> {
        self.call(|worker| worker.terminal(false)).map_err(|_| {
            self.retire();
            OwnerError::Unknown
        })
    }
    pub(crate) fn retire(&mut self) {
        self.available = false;
        let _ = self.executor.execute(|worker| worker.retire());
    }
}
fn outside_runtime() -> Result<(), OwnerError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        Err(OwnerError::Invalid)
    } else {
        Ok(())
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.executor.shutdown();
    }
}
impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Connection { redacted }")
    }
}
