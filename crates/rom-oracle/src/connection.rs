use crate::{Config, ControlTable, Deadlines, TlsWallet, Transaction, worker::Worker};
use rom_sql_core::{Executor, ExecutorError, OwnerError};
/// Private worker owns the native connection, statements and rows. No native handles escape.
pub struct Connection {
    executor: Executor<Worker>,
    bounds: Deadlines,
    available: bool,
}
impl Connection {
    /// Connect with required native TCPS, wallet trust and explicit certificate DN matching.
    /// Native TCPS trust/identity qualification is separate from the local ownership profile.
    pub fn connect(
        config: Config,
        wallet: TlsWallet,
        bounds: Deadlines,
    ) -> Result<Self, OwnerError> {
        Self::open(config, Some(wallet), bounds)
    }
    /// Explicit plaintext fixture mode; refuse remote hosts and all implicit native defaults.
    pub fn connect_loopback(config: Config, bounds: Deadlines) -> Result<Self, OwnerError> {
        if !config
            .host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|v| v.is_loopback())
        {
            return Err(OwnerError::Invalid);
        }
        Self::open(config, None, bounds)
    }
    fn open(config: Config, tls: Option<TlsWallet>, bounds: Deadlines) -> Result<Self, OwnerError> {
        outside_runtime()?;
        let executor = Executor::spawn(1, move || {
            Worker::open(config, tls, bounds).map_err(|_| ExecutorError::Initialization)
        })
        .map_err(|_| OwnerError::Unavailable)?;
        Ok(Self {
            executor,
            bounds,
            available: true,
        })
    }
    /// Native use remains admitted; this is not liveness or ownership evidence.
    pub fn is_available(&self) -> bool {
        self.available
    }
    /// Start clean READ COMMITTED. The host provisions and selects the immutable control-table identity.
    pub fn begin<'a>(&'a mut self, table: &'a ControlTable) -> Result<Transaction<'a>, OwnerError> {
        self.call(Worker::begin)?;
        Ok(Transaction::new(self, table, self.bounds.lock_seconds))
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
        if matches!(result, Err(OwnerError::Unknown)) {
            self.available = false;
        }
        result
    }
    pub(crate) fn cleanup(&mut self) -> Result<(), OwnerError> {
        self.call(|w| w.terminal(false)).map_err(|_| {
            self.retire();
            OwnerError::Unknown
        })
    }
    pub(crate) fn retire(&mut self) {
        self.available = false;
        let _ = self.executor.execute(Worker::retire);
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
