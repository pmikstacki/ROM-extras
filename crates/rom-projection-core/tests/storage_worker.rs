//! Actual checkpoint owner lifecycle and asynchronous command responses.
mod support;
use rom_projection_core::{
    Cancellation, CheckpointStore, Error, PageIntent, StorageFailure, StorageWorker,
};
use support::{Directory, cursor, initial, profile};
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}
#[test]
fn accepted_intent_survives_dropped_response_and_join_releases_ownership() {
    let dir = Directory::new();
    let worker = StorageWorker::create(dir.file(), profile(), initial()).unwrap();
    let page = PageIntent::new(&initial(), cursor(3), vec![]).unwrap();
    drop(worker.prepare_page(page.clone()).unwrap());
    worker.shutdown().unwrap();
    let store = CheckpointStore::open(&dir.file(), &profile()).unwrap();
    assert_eq!(store.load().unwrap().pending_page(), Some(&page));
    assert_eq!(
        store.load().unwrap().checkpoint().cursor("document"),
        Some(&cursor(0))
    );
    assert!(matches!(worker.load(), Err(StorageFailure::Closed)));
}
#[test]
fn typed_commands_publish_and_shutdown_precedes_another_owner() {
    let dir = Directory::new();
    let worker = StorageWorker::create(dir.file(), profile(), initial()).unwrap();
    assert!(matches!(
        CheckpointStore::open(&dir.file(), &profile()),
        Err(Error::Busy)
    ));
    let rt = runtime();
    let page = PageIntent::new(&initial(), cursor(7), vec![]).unwrap();
    rt.block_on(worker.prepare_page(page.clone()).unwrap().receive())
        .unwrap();
    let observed = page.reconcile(&profile(), vec![]).unwrap();
    rt.block_on(worker.complete_page(page, observed).unwrap().receive())
        .unwrap();
    let state = rt.block_on(worker.load().unwrap().receive()).unwrap();
    assert_eq!(state.checkpoint().cursor("document"), Some(&cursor(7)));
    drop(worker);
    drop(CheckpointStore::open(&dir.file(), &profile()).unwrap());
}
#[test]
fn cancelled_initialization_returns_the_checkpoint_category() {
    let dir = Directory::new();
    drop(CheckpointStore::create(&dir.file(), &profile(), &initial()).unwrap());
    let cancel = Cancellation::new();
    cancel.cancel();
    assert!(matches!(
        StorageWorker::open(dir.file(), profile(), cancel),
        Err(StorageFailure::Checkpoint(Error::Cancelled))
    ));
    drop(CheckpointStore::open(&dir.file(), &profile()).unwrap());
}
