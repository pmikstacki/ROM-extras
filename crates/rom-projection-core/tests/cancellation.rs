//! Actual file admission and ownership behavior under cancellation.
mod support;
use rom_projection_core::{Cancellation, CheckpointStore, Error};
use support::{Directory, initial, profile};
#[test]
fn cancelled_open_preserves_bytes_and_releases_admission() {
    let dir = Directory::new();
    drop(CheckpointStore::create(&dir.file(), &profile(), &initial()).unwrap());
    let before = std::fs::read(dir.file()).unwrap();
    let cancel = Cancellation::new();
    let clone = cancel.clone();
    std::thread::spawn(move || clone.cancel()).join().unwrap();
    assert!(matches!(
        CheckpointStore::open_cancellable(&dir.file(), &profile(), &cancel),
        Err(Error::Cancelled)
    ));
    assert_eq!(std::fs::read(dir.file()).unwrap(), before);
    let store = CheckpointStore::open(&dir.file(), &profile()).unwrap();
    assert_eq!(store.load().unwrap().checkpoint(), &initial());
}
#[test]
fn pre_cancelled_reopen_does_not_retire_a_usable_engine_or_release_ownership() {
    let dir = Directory::new();
    let mut store = CheckpointStore::create(&dir.file(), &profile(), &initial()).unwrap();
    let cancel = Cancellation::new();
    cancel.cancel();
    assert_eq!(store.reopen_cancellable(&cancel), Err(Error::Cancelled));
    assert_eq!(store.load().unwrap().checkpoint(), &initial());
    assert!(matches!(
        CheckpointStore::open(&dir.file(), &profile()),
        Err(Error::Busy)
    ));
}
