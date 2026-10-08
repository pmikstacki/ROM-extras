//! Controlled native I/O faults over a real local file, not filesystem power-loss proof.
use super::*;
use crate::CommitStatus;
use std::io;
use std::os::unix::fs::FileExt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
};

use crate::test_support as fs;

#[derive(Debug)]
struct Fault {
    mode: AtomicU8,
    fired: AtomicBool,
}
#[derive(Debug)]
struct Backend {
    file: File,
    fault: Arc<Fault>,
}
impl redb::StorageBackend for Backend {
    fn len(&self) -> io::Result<u64> {
        Ok(self.file.metadata()?.len())
    }
    fn read(&self, offset: u64, out: &mut [u8]) -> io::Result<()> {
        self.file.read_exact_at(out, offset)
    }
    fn set_len(&self, len: u64) -> io::Result<()> {
        self.file.set_len(len)
    }
    fn sync_data(&self) -> io::Result<()> {
        self.file.sync_data()?;
        if self
            .fault
            .mode
            .compare_exchange(2, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.fault.fired.store(true, Ordering::SeqCst);
            return Err(io::Error::other("controlled error after actual sync"));
        }
        if self
            .fault
            .mode
            .compare_exchange(3, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.fault.fired.store(true, Ordering::SeqCst);
            panic!("controlled unwind after actual sync");
        }
        Ok(())
    }
    fn write(&self, offset: u64, data: &[u8]) -> io::Result<()> {
        if self
            .fault
            .mode
            .compare_exchange(1, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.fault.fired.store(true, Ordering::SeqCst);
            return Err(io::Error::other("controlled error before actual write"));
        }
        self.file.write_all_at(data, offset)
    }
}

fn fixture() -> (
    fs::Directory,
    ProjectionProfile,
    CheckpointStore,
    Arc<Fault>,
) {
    let directory = fs::Directory::new();
    let profile = ProjectionProfile::new("fault-profile", "qdrant", "mapping-v1", None).unwrap();
    let initial = Checkpoint::new(
        "physical-a",
        vec![rom::JournalCursor {
            generation: "journal-a".into(),
            kind: "document".into(),
            position: 0,
        }],
    )
    .unwrap();
    drop(CheckpointStore::create(&directory.file(), &profile, &initial).unwrap());
    let owner = NativeOwnership::acquire(&directory.file(), NativeAccess::Existing).unwrap();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(owner.path())
        .unwrap();
    let fault = Arc::new(Fault {
        mode: AtomicU8::new(0),
        fired: AtomicBool::new(false),
    });
    let engine = preflight::builder()
        .create_with_backend(Backend {
            file,
            fault: fault.clone(),
        })
        .unwrap();
    let store = CheckpointStore {
        engine: Some(engine),
        owner,
        profile: profile.clone(),
    };
    (directory, profile, store, fault)
}
fn page(store: &CheckpointStore) -> PageIntent {
    PageIntent::new(
        store.load().unwrap().checkpoint(),
        rom::JournalCursor {
            generation: "journal-a".into(),
            kind: "document".into(),
            position: 1,
        },
        vec![],
    )
    .unwrap()
}
fn unknown(result: Result<TransactionId>) -> TransactionId {
    match result {
        Err(Error::Unknown { transaction }) => transaction,
        other => panic!("expected unknown native commit, received {other:?}"),
    }
}

#[test]
fn error_before_native_write_reconciles_absent_after_handle_retirement() {
    let (directory, profile, mut store, fault) = fixture();
    let page = page(&store);
    fault.mode.store(1, Ordering::SeqCst);
    let token = unknown(store.prepare_page(&page));
    assert!(fault.fired.load(Ordering::SeqCst));
    assert!(store.load().is_err());
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile),
        Err(Error::Busy)
    ));
    store.reopen().unwrap();
    assert_eq!(store.reconcile(&token).unwrap(), CommitStatus::Absent);
    assert!(store.load().unwrap().pending_page().is_none());
    assert_eq!(
        store
            .load()
            .unwrap()
            .checkpoint()
            .cursor("document")
            .unwrap()
            .position,
        0
    );
}

#[test]
fn error_after_actual_native_sync_reconciles_applied_after_handle_retirement() {
    let (_directory, _profile, mut store, fault) = fixture();
    let page = page(&store);
    fault.mode.store(2, Ordering::SeqCst);
    let token = unknown(store.prepare_page(&page));
    assert!(fault.fired.load(Ordering::SeqCst));
    assert!(store.load().is_err());
    store.reopen().unwrap();
    assert_eq!(store.reconcile(&token).unwrap(), CommitStatus::Applied);
    assert_eq!(store.load().unwrap().pending_page(), Some(&page));
    assert_eq!(
        store
            .load()
            .unwrap()
            .checkpoint()
            .cursor("document")
            .unwrap()
            .position,
        0
    );
}

#[test]
fn unwinding_commit_panic_retires_handle_and_reconciles_the_durable_intent() {
    let (_directory, _profile, mut store, fault) = fixture();
    let page = page(&store);
    fault.mode.store(3, Ordering::SeqCst);
    let token = unknown(store.prepare_page(&page));
    assert!(fault.fired.load(Ordering::SeqCst));
    assert!(store.load().is_err());
    store.reopen().unwrap();
    assert_eq!(store.reconcile(&token).unwrap(), CommitStatus::Applied);
    assert_eq!(store.load().unwrap().pending_page(), Some(&page));
}

#[test]
fn completion_sync_error_recovers_atomically_applied_keys_cursor_and_receipt() {
    completion_fault(2, CommitStatus::Applied);
}

#[test]
fn completion_write_error_recovers_atomically_absent_keys_and_unadvanced_cursor() {
    completion_fault(1, CommitStatus::Absent);
}

fn completion_fault(mode: u8, expected: CommitStatus) {
    let (_directory, profile, mut store, fault) = fixture();
    let a = OperationMetadata::new(
        rom::Key {
            kind: "document".into(),
            id: "a".into(),
        },
        1,
        7,
        false,
        [1; 32],
    )
    .unwrap();
    let b = OperationMetadata::new(
        rom::Key {
            kind: "document".into(),
            id: "b".into(),
        },
        2,
        8,
        true,
        [2; 32],
    )
    .unwrap();
    let page = PageIntent::new(
        store.load().unwrap().checkpoint(),
        rom::JournalCursor {
            generation: "journal-a".into(),
            kind: "document".into(),
            position: 2,
        },
        vec![a.clone(), b.clone()],
    )
    .unwrap();
    let prepare = store.prepare_page(&page).unwrap();
    let observations = vec![
        RemoteObservation::new(&profile, "physical-a", a.clone()).unwrap(),
        RemoteObservation::new(&profile, "physical-a", b.clone()).unwrap(),
    ];
    let observed = page.reconcile(&profile, observations).unwrap();
    fault.mode.store(mode, Ordering::SeqCst);
    let token = unknown(store.complete_page(&page, &observed));
    assert!(fault.fired.load(Ordering::SeqCst));
    assert!(store.load().is_err());
    let token = TransactionId::decode(&token.encode()).unwrap();
    store.reopen().unwrap();
    assert_eq!(store.reconcile(&token).unwrap(), expected);
    assert_eq!(store.reconcile(&prepare).unwrap(), CommitStatus::Applied);
    let state = store.load().unwrap();
    match expected {
        CommitStatus::Applied => {
            assert!(state.pending_page().is_none());
            assert_eq!(state.checkpoint().cursor("document").unwrap().position, 2);
            assert_eq!(store.key_state(a.key()).unwrap().unwrap().revision(), 7);
            assert!(store.key_state(b.key()).unwrap().unwrap().is_tombstone());
        }
        CommitStatus::Absent => {
            assert_eq!(state.pending_page(), Some(&page));
            assert_eq!(state.checkpoint().cursor("document").unwrap().position, 0);
            assert!(store.key_state(a.key()).unwrap().is_none());
            assert!(store.key_state(b.key()).unwrap().is_none());
        }
    }
}
