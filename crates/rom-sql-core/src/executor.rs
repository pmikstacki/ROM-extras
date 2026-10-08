use crate::{ExecutorError, Ticket};
use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, TrySendError, sync_channel},
    },
    thread::{self, JoinHandle, ThreadId},
};

type Job<C> = Box<dyn FnOnce(&mut C) -> bool + Send + 'static>;

struct Inner<C> {
    sender: Mutex<Option<SyncSender<Job<C>>>>,
    worker: ThreadId,
    healthy: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
}

struct WorkerHealth(Arc<AtomicBool>);
impl Drop for WorkerHealth {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// A bounded, connection-owning executor with one dedicated worker thread.
///
/// A connection is created and destroyed on that worker; it need not be Send.
/// Cloned handles share the worker and queue. Capacity counts queued operations,
/// excluding the currently executing operation. Captured payload sizes are the
/// driver's responsibility. This executor does not bound arbitrary closure bytes.
///
/// Use [`Self::shutdown`] to close admission, drain queued work, and join.
/// Dropping the last handle closes admission and detaches the worker. Already
/// admitted work continues. Neither drop nor close cancels database operations.
pub struct Executor<C> {
    inner: Arc<Inner<C>>,
}

impl<C> Clone for Executor<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<C: 'static> Executor<C> {
    /// Initialize a connection on a new worker, with a queue of `capacity` jobs.
    ///
    /// Initialization is synchronous and must have driver-level connect deadlines.
    /// Capacity must be in 1..=65_536. A failed or panicking factory admits no jobs.
    pub fn spawn(
        capacity: usize,
        initialize: impl FnOnce() -> Result<C, ExecutorError> + Send + 'static,
    ) -> Result<Self, ExecutorError> {
        if !(1..=65_536).contains(&capacity) {
            return Err(ExecutorError::InvalidCapacity);
        }
        let (sender, receiver) = sync_channel::<Job<C>>(capacity);
        let (ready_sender, ready_receiver) = sync_channel(1);
        let healthy = Arc::new(AtomicBool::new(true));
        let worker_health = healthy.clone();
        let join = thread::Builder::new()
            .name("rom-sql-connection".into())
            .spawn(move || {
                let _health = WorkerHealth(worker_health);
                let mut connection = match catch_unwind(AssertUnwindSafe(initialize)) {
                    Ok(Ok(connection)) => connection,
                    _ => {
                        let _ = ready_sender.send(Err(ExecutorError::Initialization));
                        return;
                    }
                };
                if ready_sender.send(Ok(thread::current().id())).is_err() {
                    return;
                }
                while let Ok(job) = receiver.recv() {
                    if !job(&mut connection) {
                        break;
                    }
                }
            })
            .map_err(|_| ExecutorError::Initialization)?;
        match ready_receiver.recv() {
            Ok(Ok(worker)) => Ok(Self {
                inner: Arc::new(Inner {
                    sender: Mutex::new(Some(sender)),
                    worker,
                    healthy,
                    join: Mutex::new(Some(join)),
                }),
            }),
            _ => {
                let _ = join.join();
                Err(ExecutorError::Initialization)
            }
        }
    }

    /// Admit an operation without waiting for queue space or its result.
    ///
    /// An unwinding panic returns `Unknown` and retires the connection. Queued
    /// operations then return `Closed` without executing. Aborting panics and
    /// process failures require durable provider-level recovery.
    pub fn submit<R: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut C) -> R + Send + 'static,
    ) -> Result<Ticket<R>, ExecutorError> {
        self.check_caller()?;
        if !self.inner.healthy.load(Ordering::Acquire) {
            return Err(ExecutorError::Closed);
        }
        let (response_sender, response) = sync_channel(1);
        let healthy = self.inner.healthy.clone();
        let job: Job<C> = Box::new(move |connection| {
            match catch_unwind(AssertUnwindSafe(|| operation(connection))) {
                Ok(result) => {
                    let _ = response_sender.send(Ok(result));
                    true
                }
                Err(_) => {
                    healthy.store(false, Ordering::Release);
                    let _ = response_sender.send(Err(ExecutorError::Unknown));
                    false
                }
            }
        });
        // Drop refused closures outside the admission lock. Captures may own
        // handles or driver values with user-defined destructors.
        let admitted = {
            let guard = self
                .inner
                .sender
                .lock()
                .map_err(|_| ExecutorError::Closed)?;
            match guard.as_ref() {
                Some(sender) => sender.try_send(job),
                None => return Err(ExecutorError::Closed),
            }
        };
        match admitted {
            Ok(()) => Ok(Ticket {
                response,
                worker: self.inner.worker,
                consumed: Cell::new(false),
            }),
            Err(TrySendError::Full(_)) => Err(ExecutorError::Overloaded),
            Err(TrySendError::Disconnected(_)) => Err(ExecutorError::Closed),
        }
    }

    /// Submit an operation and block until its terminal response.
    ///
    /// Driver adapters must call this from a blocking execution context.
    /// Async Runtime workers should use their host's bounded blocking bridge.
    pub fn execute<R: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut C) -> R + Send + 'static,
    ) -> Result<R, ExecutorError> {
        self.submit(operation)?.wait()
    }

    /// Close admission for all cloned handles. Accepted work keeps running.
    pub fn close(&self) -> Result<(), ExecutorError> {
        self.inner
            .sender
            .lock()
            .map_err(|_| ExecutorError::Closed)?
            .take();
        Ok(())
    }

    /// Close admission, drain queued work, and wait for connection destruction.
    ///
    /// This can block while a driver operation runs. Driver deadlines are required;
    /// this method cannot force rollback or safely interrupt arbitrary Rust code.
    pub fn shutdown(&self) -> Result<(), ExecutorError> {
        self.check_caller()?;
        self.close()?;
        let mut guard = self.inner.join.lock().map_err(|_| ExecutorError::Unknown)?;
        if let Some(worker) = guard.take() {
            worker.join().map_err(|_| ExecutorError::Unknown)?;
        }
        Ok(())
    }

    fn check_caller(&self) -> Result<(), ExecutorError> {
        if thread::current().id() == self.inner.worker {
            Err(ExecutorError::Reentrant)
        } else {
            Ok(())
        }
    }
}
