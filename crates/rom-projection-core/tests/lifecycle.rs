//! Actual checkpoint owner lifecycle with independent asynchronous progress.
mod support;
use rom_projection_core::{Cancellation, CheckpointStore, Error, StorageFailure, StorageLifecycle};
use support::{Directory, initial, profile};
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}
#[test]
fn managed_create_shutdown_and_reopen_release_actual_native_ownership() {
    let directory = Directory::new();
    let host = StorageLifecycle::new().unwrap();
    let rt = runtime();
    let worker = rt
        .block_on(
            host.create(directory.file(), profile(), initial(), Cancellation::new())
                .unwrap()
                .receive(),
        )
        .unwrap();
    assert_eq!(
        rt.block_on(worker.load().unwrap().receive())
            .unwrap()
            .checkpoint(),
        &initial()
    );
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Busy)
    ));
    rt.block_on(worker.shutdown_async()).unwrap();
    drop(CheckpointStore::open(&directory.file(), &profile()).unwrap());
    let second = rt
        .block_on(
            host.open(directory.file(), profile(), Cancellation::new())
                .unwrap()
                .receive(),
        )
        .unwrap();
    rt.block_on(second.shutdown_async()).unwrap();
    host.shutdown().unwrap();
}
#[test]
fn active_owner_rejects_second_start_without_creating_its_file() {
    let directory = Directory::new();
    let host = StorageLifecycle::new().unwrap();
    let rt = runtime();
    let worker = rt
        .block_on(
            host.create(directory.file(), profile(), initial(), Cancellation::new())
                .unwrap()
                .receive(),
        )
        .unwrap();
    let other = directory.file().with_file_name("not-admitted.redb");
    assert!(matches!(
        host.create(other.clone(), profile(), initial(), Cancellation::new()),
        Err(StorageFailure::Overloaded)
    ));
    assert!(!other.exists());
    rt.block_on(worker.shutdown_async()).unwrap();
}
#[test]
fn precancelled_create_never_creates_checkpoint() {
    let directory = Directory::new();
    let host = StorageLifecycle::new().unwrap();
    let cancel = Cancellation::new();
    cancel.cancel();
    assert!(matches!(
        host.create(directory.file(), profile(), initial(), cancel),
        Err(StorageFailure::Checkpoint(Error::Cancelled))
    ));
    assert!(!directory.file().exists());
}
#[test]
fn host_shutdown_revokes_a_live_managed_owner_and_closes_future_admission() {
    let directory = Directory::new();
    let host = StorageLifecycle::new().unwrap();
    let rt = runtime();
    let worker = rt
        .block_on(
            host.create(directory.file(), profile(), initial(), Cancellation::new())
                .unwrap()
                .receive(),
        )
        .unwrap();
    host.shutdown().unwrap();
    rt.block_on(worker.shutdown_async()).unwrap();
    assert!(matches!(worker.load(), Err(StorageFailure::Closed)));
    assert!(matches!(
        host.open(directory.file(), profile(), Cancellation::new()),
        Err(StorageFailure::Closed)
    ));
    drop(CheckpointStore::open(&directory.file(), &profile()).unwrap());
}
#[test]
fn oversized_path_rejects_before_lifecycle_admission() {
    let host = StorageLifecycle::new().unwrap();
    assert!(matches!(
        host.create(
            "x".repeat(4097).into(),
            profile(),
            initial(),
            Cancellation::new()
        ),
        Err(StorageFailure::Checkpoint(Error::TooLarge))
    ));
}

#[test]
fn failed_native_start_preserves_classification_and_releases_only_its_slot() {
    let directory = Directory::new();
    let standalone = CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    let host = StorageLifecycle::new().unwrap();
    let rt = runtime();
    assert!(matches!(
        rt.block_on(
            host.open(directory.file(), profile(), Cancellation::new())
                .unwrap()
                .receive()
        ),
        Err(StorageFailure::Checkpoint(Error::Busy))
    ));
    drop(standalone);
    let worker = rt
        .block_on(
            host.open(directory.file(), profile(), Cancellation::new())
                .unwrap()
                .receive(),
        )
        .unwrap();
    fn require_send<T: Send>(value: T) -> T {
        value
    }
    rt.block_on(require_send(worker.shutdown_async())).unwrap();
}
