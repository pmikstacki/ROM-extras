//! Deterministic cancellation at actual file-copy and key-scan boundaries.
use super::*;
use crate::test_support as support;
use crate::{Cancellation, CheckpointStore};
use support::{Directory, cursor, initial, profile};

#[test]
fn scan_cancels_at_256_records_before_later_corruption_is_examined() {
    let dir = Directory::new();
    let mut store = CheckpointStore::create(&dir.file(), &profile(), &initial()).unwrap();
    let page = PageIntent::new(&initial(), cursor(1), vec![]).unwrap();
    store.prepare_page(&page).unwrap();
    let observed = page.reconcile(&profile(), vec![]).unwrap();
    store.complete_page(&page, &observed).unwrap();
    drop(store);
    let db = builder().open(dir.file()).unwrap();
    let write = db.begin_write().unwrap();
    {
        let mut keys = write.open_table(KEYS).unwrap();
        for n in 0..300 {
            let key = codec::key(&rom::Key {
                kind: "document".into(),
                id: format!("key-{n:04}"),
            });
            let value = if n == 299 {
                vec![0]
            } else {
                codec::key_state(&KeyState {
                    revision: 1,
                    tombstone: false,
                    digest: [0; 32],
                })
            };
            keys.insert(key.as_slice(), value.as_slice()).unwrap();
        }
    }
    write.commit().unwrap();
    let transaction = db.begin_read().unwrap();
    let mut checkpoints = 0;
    let cancel = Cancellation::new();
    let result = validate_cancellable(&transaction, &profile(), &mut || {
        checkpoints += 1;
        if checkpoints == 2 {
            cancel.cancel();
        }
        cancel.check()
    });
    assert_eq!(result, Err(Error::Cancelled));
    assert_eq!(checkpoints, 2);
    assert!(validate_cancellable(&transaction, &profile(), &mut || Ok(())).is_err());
}

#[test]
fn bounded_copy_cancels_after_one_chunk_without_changing_source() {
    let dir = Directory::new();
    let bytes = vec![42; 150 * 1024];
    std::fs::write(dir.file(), &bytes).unwrap();
    let mut source = File::open(dir.file()).unwrap();
    let mut output = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(dir.0.join("copy"))
        .unwrap();
    let cancel = Cancellation::new();
    let mut chunks = 0;
    let result = copy_contents(&mut source, &mut output, bytes.len() as u64, &mut || {
        chunks += 1;
        if chunks == 2 {
            cancel.cancel();
        }
        cancel.check()
    });
    assert_eq!(result, Err(Error::Cancelled));
    assert_eq!(output.metadata().unwrap().len(), 64 * 1024);
    assert_eq!(std::fs::read(dir.file()).unwrap(), bytes);
}
