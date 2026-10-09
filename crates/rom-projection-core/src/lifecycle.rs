//! One-slot host supervisor retaining native cleanup across abandoned async responses.
use crate::{Cancellation, Checkpoint, Error, ProjectionProfile, StorageFailure, StorageWorker};
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, mpsc},
    thread::JoinHandle,
};
type Result<T> = std::result::Result<T, StorageFailure>;
struct State {
    sender: Option<mpsc::SyncSender<Start>>,
    active: Option<Arc<Lease>>,
}
struct Shared {
    state: Mutex<State>,
}
/// Fixed one-owner lifecycle bridge. Create and finally join this host outside async execution.
/// Managed owner start, drop and shutdown waiting do not join native I/O on async callers.
/// A live or cleaning owner occupies the sole slot; another start returns Overloaded.
/// Native I/O has no hard deadline. Host shutdown revokes live managed owners and drains admitted work.
pub struct StorageLifecycle {
    shared: Arc<Shared>,
    join: Mutex<Option<JoinHandle<()>>>,
}
/// An admitted start response. Dropping it requests cooperative cleanup, not native rollback.
pub struct StorageStartup {
    receiver: Option<tokio::sync::oneshot::Receiver<Result<StorageWorker>>>,
    lease: Arc<Lease>,
    transferred: bool,
}
impl StorageStartup {
    /// Receive the managed owner without blocking native initialization on this caller.
    pub async fn receive(mut self) -> Result<StorageWorker> {
        let result = self
            .receiver
            .take()
            .ok_or(StorageFailure::Unclassified)?
            .await
            .map_err(|_| StorageFailure::Unclassified)?;
        if result.is_ok() {
            self.transferred = true;
        }
        result
    }
}
impl Drop for StorageStartup {
    fn drop(&mut self) {
        if !self.transferred {
            self.lease.close();
        }
    }
}
struct Start {
    request: Request,
    lease: Arc<Lease>,
    response: tokio::sync::oneshot::Sender<Result<StorageWorker>>,
}
enum Request {
    #[cfg(test)]
    Test(Box<dyn FnOnce() -> Result<StorageWorker> + Send>),
    Create(PathBuf, ProjectionProfile, Checkpoint),
    Open(PathBuf, ProjectionProfile),
}
impl Request {
    fn initialize(self, cancel: &Cancellation) -> Result<StorageWorker> {
        cancel.check().map_err(StorageFailure::Checkpoint)?;
        match self {
            #[cfg(test)]
            Self::Test(factory) => factory(),
            Self::Create(path, profile, initial) => StorageWorker::create(path, profile, initial),
            Self::Open(path, profile) => StorageWorker::open(path, profile, cancel.clone()),
        }
    }
}
struct LeaseState {
    close: bool,
    result: Option<Result<()>>,
}
pub(crate) struct Lease {
    cancel: Cancellation,
    state: Mutex<LeaseState>,
    changed: Condvar,
    completion: tokio::sync::watch::Sender<Option<Result<()>>>,
}
impl Lease {
    fn new(cancel: Cancellation) -> Self {
        let (completion, _) = tokio::sync::watch::channel(None);
        Self {
            cancel,
            state: Mutex::new(LeaseState {
                close: false,
                result: None,
            }),
            changed: Condvar::new(),
            completion,
        }
    }
    pub(crate) fn close(&self) {
        self.cancel.cancel();
        self.state.lock().unwrap_or_else(|e| e.into_inner()).close = true;
        self.changed.notify_all();
    }
    fn wait_close(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while !state.close {
            state = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
        }
    }
    fn complete(&self, result: Result<()>) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).result = Some(result.clone());
        self.completion.send_replace(Some(result));
        self.changed.notify_all();
    }
    pub(crate) fn wait(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(result) = &state.result {
                return result.clone();
            }
            state = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
        }
    }
    pub(crate) async fn receive(&self) -> Result<()> {
        let mut receiver = self.completion.subscribe();
        loop {
            if let Some(result) = receiver.borrow_and_update().clone() {
                return result;
            }
            receiver
                .changed()
                .await
                .map_err(|_| StorageFailure::Unclassified)?;
        }
    }
}
impl StorageLifecycle {
    /// Start one persistent bridge thread; call from the embedding host's blocking setup context.
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                sender: Some(sender),
                active: None,
            }),
        });
        let worker_shared = shared.clone();
        let join = std::thread::Builder::new()
            .name("rom-projection-lifecycle".into())
            .spawn(move || supervise(worker_shared, receiver))
            .map_err(|_| StorageFailure::Unclassified)?;
        Ok(Self {
            shared,
            join: Mutex::new(Some(join)),
        })
    }
    /// Admit bounded checkpoint creation without waiting for native I/O or another owner.
    /// Cancellation cannot erase a checkpoint whose creation already completed.
    pub fn create(
        &self,
        path: PathBuf,
        profile: ProjectionProfile,
        initial: Checkpoint,
        cancel: Cancellation,
    ) -> Result<StorageStartup> {
        Self::path(&path)?;
        self.admit(Request::Create(path, profile, initial), cancel)
    }
    /// Admit cancellable checkpoint opening without waiting for native I/O or another owner.
    pub fn open(
        &self,
        path: PathBuf,
        profile: ProjectionProfile,
        cancel: Cancellation,
    ) -> Result<StorageStartup> {
        Self::path(&path)?;
        self.admit(Request::Open(path, profile), cancel)
    }
    fn path(path: &std::path::Path) -> Result<()> {
        if path.as_os_str().as_encoded_bytes().len() > 4096 {
            return Err(StorageFailure::Checkpoint(Error::TooLarge));
        }
        Ok(())
    }
    fn admit(&self, request: Request, cancel: Cancellation) -> Result<StorageStartup> {
        cancel.check().map_err(StorageFailure::Checkpoint)?;
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| StorageFailure::Unclassified)?;
        let sender = state.sender.as_ref().ok_or(StorageFailure::Closed)?;
        if state.active.is_some() {
            return Err(StorageFailure::Overloaded);
        }
        let lease = Arc::new(Lease::new(cancel));
        let (response, receiver) = tokio::sync::oneshot::channel();
        sender
            .try_send(Start {
                request,
                lease: lease.clone(),
                response,
            })
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => StorageFailure::Overloaded,
                mpsc::TrySendError::Disconnected(_) => StorageFailure::Closed,
            })?;
        state.active = Some(lease.clone());
        Ok(StorageStartup {
            receiver: Some(receiver),
            lease,
            transferred: false,
        })
    }
    /// Close start admission, revoke the current owner, drain native work and join the supervisor.
    /// Blocking embedding context required; an observation timeout is not forced termination.
    pub fn shutdown(&self) -> Result<()> {
        {
            let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
            state.sender.take();
            if let Some(lease) = &state.active {
                lease.close();
            }
        }
        if let Some(join) = self
            .join
            .lock()
            .map_err(|_| StorageFailure::Unclassified)?
            .take()
        {
            join.join().map_err(|_| StorageFailure::Unclassified)?;
        }
        Ok(())
    }
}
impl Drop for StorageLifecycle {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
fn supervise(shared: Arc<Shared>, receiver: mpsc::Receiver<Start>) {
    while let Ok(start) = receiver.recv() {
        let Start {
            request,
            lease,
            response,
        } = start;
        let mut response = Some(response);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            request.initialize(&lease.cancel)
        }))
        .unwrap_or(Err(StorageFailure::Unclassified));
        let finished = match result {
            Ok(mut owner) => {
                if lease.cancel.is_cancelled() {
                    drop(owner); // Standalone cleanup joins here, never on the async receiver.
                    let failure = StorageFailure::Checkpoint(Error::Cancelled);
                    Err(failure)
                } else {
                    let executor = owner.manage(lease.clone());
                    // A dropped receiver disposes the managed owner and requests supervisor cleanup.
                    let _ = response
                        .take()
                        .expect("single startup response")
                        .send(Ok(owner));
                    lease.wait_close();
                    executor
                        .shutdown()
                        .map_err(super::storage_worker::admission_error)
                }
            }
            Err(error) => Err(error),
        };
        shared
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .active = None;
        lease.complete(finished.clone());
        // A failed start is reusable before its response wakes another admission attempt.
        if let Some(response) = response {
            let _ = response.send(Err(finished.err().unwrap_or(StorageFailure::Unclassified)));
        }
    }
}

#[cfg(test)]
#[path = "lifecycle_start_tests.rs"]
mod tests;
