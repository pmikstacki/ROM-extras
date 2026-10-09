//! Controlled active native jobs in real owned checkpoint files.
use super::*;
use crate::{
    StorageLifecycle,
    test_support::{Directory, cursor, initial, profile},
};
use std::{future::Future, sync::mpsc, time::Duration};
const WAIT: Duration = Duration::from_secs(5);
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}
fn owner(host: &StorageLifecycle, directory: &Directory) -> StorageWorker {
    runtime()
        .block_on(
            host.create(directory.file(), profile(), initial(), Cancellation::new())
                .unwrap()
                .receive(),
        )
        .unwrap()
}
fn block(worker: &StorageWorker) -> mpsc::SyncSender<()> {
    let (entered, entry) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    drop(
        worker
            .submit(move |_| {
                entered.send(()).unwrap();
                released.recv_timeout(WAIT).unwrap();
                Ok(())
            })
            .unwrap(),
    );
    entry.recv_timeout(WAIT).unwrap();
    release
}
#[test]
fn async_shutdown_yields_while_native_work_is_still_active() {
    let directory = Directory::new();
    let host = StorageLifecycle::new().unwrap();
    let worker = owner(&host, &directory);
    let release = block(&worker);
    let rt = runtime();
    let mut shutdown = Box::pin(worker.shutdown_async());
    rt.block_on(std::future::poll_fn(|cx| {
        assert!(shutdown.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    }));
    assert_eq!(rt.block_on(async { 7 }), 7);
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Busy)
    ));
    release.send(()).unwrap();
    rt.block_on(shutdown).unwrap();
    drop(CheckpointStore::open(&directory.file(), &profile()).unwrap());
}
#[test]
fn dropped_shutdown_future_still_drains_accepted_intent_and_retains_terminal_result() {
    let directory = Directory::new();
    let host = StorageLifecycle::new().unwrap();
    let worker = owner(&host, &directory);
    let release = block(&worker);
    let page = PageIntent::new(&initial(), cursor(3), vec![]).unwrap();
    drop(worker.prepare_page(page.clone()).unwrap());
    let rt = runtime();
    let mut shutdown = Box::pin(worker.shutdown_async());
    rt.block_on(std::future::poll_fn(|cx| {
        assert!(shutdown.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    }));
    drop(shutdown);
    assert!(matches!(
        host.open(directory.file(), profile(), Cancellation::new()),
        Err(StorageFailure::Overloaded)
    ));
    release.send(()).unwrap();
    rt.block_on(worker.shutdown_async()).unwrap();
    rt.block_on(worker.shutdown_async()).unwrap();
    let store = CheckpointStore::open(&directory.file(), &profile()).unwrap();
    assert_eq!(store.load().unwrap().pending_page(), Some(&page));
}
#[test]
fn managed_drop_never_joins_active_native_io_on_the_async_caller() {
    let directory = Directory::new();
    let host = StorageLifecycle::new().unwrap();
    let worker = owner(&host, &directory);
    let completion = worker.lifecycle.as_ref().unwrap().clone();
    let release = block(&worker);
    runtime().block_on(async {
        drop(worker);
    });
    assert!(matches!(
        host.open(directory.file(), profile(), Cancellation::new()),
        Err(StorageFailure::Overloaded)
    ));
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Busy)
    ));
    release.send(()).unwrap();
    runtime().block_on(completion.receive()).unwrap();
    drop(CheckpointStore::open(&directory.file(), &profile()).unwrap());
}
