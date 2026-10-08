//! Deterministic admission and owner destruction with real native storage.
use super::*;
use crate::test_support::{Directory, cursor, initial, profile};
use std::{sync::mpsc, time::Duration};
const WAIT: Duration = Duration::from_secs(5);
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}
#[test]
fn full_queue_rejects_without_execution_and_join_drains_the_accepted_intent() {
    let directory = Directory::new();
    let worker = StorageWorker::create(directory.file(), profile(), initial()).unwrap();
    let (entered, entry) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    let blocked = worker
        .submit(move |_| {
            entered.send(()).unwrap();
            released.recv_timeout(WAIT).unwrap();
            Ok(())
        })
        .unwrap();
    entry.recv_timeout(WAIT).unwrap();
    let page = PageIntent::new(&initial(), cursor(3), vec![]).unwrap();
    let accepted = worker.prepare_page(page.clone()).unwrap();
    assert!(matches!(worker.load(), Err(StorageFailure::Overloaded)));
    drop(accepted);
    release.send(()).unwrap();
    worker.shutdown().unwrap();
    runtime().block_on(blocked.receive()).unwrap();
    let store = CheckpointStore::open(&directory.file(), &profile()).unwrap();
    assert_eq!(store.load().unwrap().pending_page(), Some(&page));
}
#[test]
fn owner_drop_waits_for_active_work_and_releases_native_ownership() {
    let directory = Directory::new();
    let worker = StorageWorker::create(directory.file(), profile(), initial()).unwrap();
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
    let (started, start) = mpsc::sync_channel(1);
    let (done, completion) = mpsc::sync_channel(1);
    let dropping = std::thread::spawn(move || {
        started.send(()).unwrap();
        drop(worker);
        done.send(()).unwrap();
    });
    start.recv_timeout(WAIT).unwrap();
    assert!(completion.try_recv().is_err());
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Busy)
    ));
    release.send(()).unwrap();
    completion.recv_timeout(WAIT).unwrap();
    dropping.join().unwrap();
    drop(CheckpointStore::open(&directory.file(), &profile()).unwrap());
}
#[test]
fn unexpected_job_panic_loses_response_without_claiming_rollback() {
    let directory = Directory::new();
    let worker = StorageWorker::create(directory.file(), profile(), initial()).unwrap();
    let response = worker
        .submit::<()>(|_| panic!("controlled non-commit job panic"))
        .unwrap();
    assert_eq!(
        runtime().block_on(response.receive()),
        Err(StorageFailure::Unclassified)
    );
    worker.shutdown().unwrap();
    assert!(matches!(worker.load(), Err(StorageFailure::Closed)));
    drop(CheckpointStore::open(&directory.file(), &profile()).unwrap());
}
