//! Abandoned buffered startup ownership and actual cleanup.
use super::*;
use crate::test_support::{Directory, initial, profile};
#[test]
fn dropping_a_buffered_startup_response_releases_ownership_without_host_shutdown() {
    let directory = Directory::new();
    let host = StorageLifecycle::new().unwrap();
    let startup = host
        .create(directory.file(), profile(), initial(), Cancellation::new())
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while startup.receiver.as_ref().unwrap().is_empty() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let completion = startup.lease.clone();
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(async {
        drop(startup);
    });
    rt.block_on(completion.receive()).unwrap();
    let worker = rt
        .block_on(
            host.open(directory.file(), profile(), Cancellation::new())
                .unwrap()
                .receive(),
        )
        .unwrap();
    rt.block_on(worker.shutdown_async()).unwrap();
}

#[test]
fn dropped_inflight_startup_future_keeps_slot_until_actual_owner_cleanup() {
    use std::future::Future;
    let directory = Directory::new();
    let host = StorageLifecycle::new().unwrap();
    let (entered, entry) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    let path = directory.file();
    let startup = host
        .admit(
            Request::Test(Box::new(move || {
                let owner = StorageWorker::create(path, profile(), initial())?;
                entered.send(()).unwrap();
                released
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                Ok(owner)
            })),
            Cancellation::new(),
        )
        .unwrap();
    let completion = startup.lease.clone();
    let mut response = Box::pin(startup.receive());
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(std::future::poll_fn(|cx| {
        assert!(response.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    }));
    entry
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    drop(response);
    assert!(matches!(
        host.open(directory.file(), profile(), Cancellation::new()),
        Err(StorageFailure::Overloaded)
    ));
    assert!(matches!(
        crate::CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Busy)
    ));
    release.send(()).unwrap();
    assert_eq!(
        rt.block_on(completion.receive()),
        Err(StorageFailure::Checkpoint(Error::Cancelled))
    );
    assert!(directory.file().exists()); // Cancelled response does not roll back accepted creation.
    drop(crate::CheckpointStore::open(&directory.file(), &profile()).unwrap());
}

#[test]
fn native_start_failure_releases_slot_before_waking_its_response() {
    use std::{
        future::Future,
        task::{Context, Wake, Waker},
        time::Duration,
    };
    struct Gate {
        entered: mpsc::SyncSender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }
    impl Wake for Gate {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }
    }
    let directory = Directory::new();
    let existing =
        crate::CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    let host = StorageLifecycle::new().unwrap();
    let (proceed, permitted) = mpsc::sync_channel(1);
    let path = directory.file();
    let mut startup = host
        .admit(
            Request::Test(Box::new(move || {
                permitted.recv_timeout(Duration::from_secs(5)).unwrap();
                StorageWorker::open(path, profile(), Cancellation::new())
            })),
            Cancellation::new(),
        )
        .unwrap();
    let (entered, entry) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    let waker = Waker::from(Arc::new(Gate {
        entered,
        release: Mutex::new(released),
    }));
    assert!(
        std::pin::Pin::new(startup.receiver.as_mut().unwrap())
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    proceed.send(()).unwrap();
    entry.recv_timeout(Duration::from_secs(5)).unwrap();
    let available = host.shared.state.lock().unwrap().active.is_none();
    release.send(()).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    assert!(matches!(
        rt.block_on(startup.receive()),
        Err(StorageFailure::Checkpoint(Error::Busy))
    ));
    assert!(
        available,
        "failure response woke before lifecycle admission was released"
    );
    drop(existing);
}
