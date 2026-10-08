//! Process interruption and source-preserving format admission.
mod support;
use rom_projection_core::{
    CheckpointStore, Error, OperationMetadata, PageIntent, RemoteObservation,
};
use std::process::{Command, Stdio};
use support::{Directory, cursor, initial, profile};

fn child(path: &std::path::Path, mode: &str) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "child_process", "--nocapture"])
        .env("ROM_EXTRAS_CHECKPOINT_CHILD_PATH", path)
        .env("ROM_EXTRAS_CHECKPOINT_CHILD_MODE", mode)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            return;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("checkpoint child exceeded deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
#[ignore = "invoked in a bounded child process by the parent cases"]
fn child_process() {
    let path =
        std::path::PathBuf::from(std::env::var_os("ROM_EXTRAS_CHECKPOINT_CHILD_PATH").unwrap());
    match std::env::var("ROM_EXTRAS_CHECKPOINT_CHILD_MODE")
        .unwrap()
        .as_str()
    {
        "prepare" => {
            let mut store = CheckpointStore::open(&path, &profile()).unwrap();
            let page = PageIntent::new(&initial(), cursor(3), vec![]).unwrap();
            store.prepare_page(&page).unwrap();
            // Deliberately bypass Drop after the acknowledged intent commit.
            std::process::exit(0);
        }
        "contend" => {
            assert!(matches!(
                CheckpointStore::open(&path, &profile()),
                Err(Error::Busy)
            ));
        }
        "future" => {
            let _database = future_format(&path);
            std::process::exit(0);
        }
        _ => panic!("invalid child mode"),
    }
}

#[test]
fn acknowledged_intent_recovers_after_process_exit_without_engine_drop() {
    let directory = Directory::new();
    drop(CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap());
    child(&directory.file(), "prepare");
    assert!(matches!(
        redb::Database::builder().open_read_only(directory.file()),
        Err(redb::DatabaseError::RepairAborted)
    ));
    let reopened = CheckpointStore::open(&directory.file(), &profile()).unwrap();
    assert_eq!(
        reopened.load().unwrap().checkpoint().cursor("document"),
        Some(&cursor(0))
    );
    assert_eq!(
        reopened
            .load()
            .unwrap()
            .pending_page()
            .unwrap()
            .next_cursor(),
        &cursor(3)
    );
}

#[test]
fn ownership_rejects_a_second_process_and_supported_path_alias() {
    let directory = Directory::new();
    let store = CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    child(&directory.file(), "contend");
    let alias = directory.0.join("alias.redb");
    std::os::unix::fs::symlink(directory.file(), &alias).unwrap();
    assert!(matches!(
        CheckpointStore::open(&alias, &profile()),
        Err(Error::Busy)
    ));
    drop(store);
    assert!(CheckpointStore::open(&alias, &profile()).is_ok());
}

#[test]
fn future_format_rejection_preserves_original_bytes() {
    let directory = Directory::new();
    drop(CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap());
    drop(future_format(&directory.file()));
    let before = std::fs::read(directory.file()).unwrap();
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Unsupported)
    ));
    assert_eq!(std::fs::read(directory.file()).unwrap(), before);
}

fn future_format(path: &std::path::Path) -> redb::Database {
    let database = redb::Database::open(path).unwrap();
    let transaction = database.begin_write().unwrap();
    {
        let definition = redb::TableDefinition::<&[u8], &[u8]>::new("romx_projection_meta");
        let mut table = transaction.open_table(definition).unwrap();
        use redb::ReadableTable;
        let mut marker = table
            .get(b"profile".as_slice())
            .unwrap()
            .unwrap()
            .value()
            .to_vec();
        marker[7..9].copy_from_slice(&2u16.to_be_bytes());
        table
            .insert(b"profile".as_slice(), marker.as_slice())
            .unwrap();
    }
    transaction.commit().unwrap();
    database
}

#[test]
fn rejected_future_dirty_source_is_not_repaired_or_rewritten() {
    let directory = Directory::new();
    drop(CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap());
    child(&directory.file(), "future");
    assert!(matches!(
        redb::Database::builder().open_read_only(directory.file()),
        Err(redb::DatabaseError::RepairAborted)
    ));
    let before = std::fs::read(directory.file()).unwrap();
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Unsupported)
    ));
    assert_eq!(std::fs::read(directory.file()).unwrap(), before);
    assert!(!std::fs::read_dir(&directory.0).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".romx-projection-preflight-")
    }));
}

fn completed_file() -> Directory {
    let directory = Directory::new();
    let mut store = CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    let operation = OperationMetadata::new(
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
    let page = PageIntent::new(&initial(), cursor(1), vec![operation.clone()]).unwrap();
    store.prepare_page(&page).unwrap();
    let observed = page
        .reconcile(
            &profile(),
            vec![RemoteObservation::new(&profile(), "physical-a", operation).unwrap()],
        )
        .unwrap();
    store.complete_page(&page, &observed).unwrap();
    drop(store);
    directory
}

#[test]
fn retained_receipt_cannot_hide_a_changed_completed_cursor() {
    let directory = completed_file();
    let database = redb::Database::open(directory.file()).unwrap();
    let transaction = database.begin_write().unwrap();
    {
        use redb::ReadableTable;
        let definition = redb::TableDefinition::<&[u8], &[u8]>::new("romx_projection_state");
        let mut table = transaction.open_table(definition).unwrap();
        let mut state = table
            .get(b"checkpoint".as_slice())
            .unwrap()
            .unwrap()
            .value()
            .to_vec();
        // Header10 + target(2+10) + sequence8 + count2 + kind(2+8) + generation(2+9).
        state[53..61].copy_from_slice(&99u64.to_be_bytes());
        table
            .insert(b"checkpoint".as_slice(), state.as_slice())
            .unwrap();
    }
    transaction.commit().unwrap();
    drop(database);
    let before = std::fs::read(directory.file()).unwrap();
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Corrupt)
    ));
    assert_eq!(std::fs::read(directory.file()).unwrap(), before);
}

#[test]
fn retained_receipt_cannot_hide_a_missing_acknowledged_key() {
    let directory = completed_file();
    let database = redb::Database::open(directory.file()).unwrap();
    let transaction = database.begin_write().unwrap();
    {
        let definition = redb::TableDefinition::<&[u8], &[u8]>::new("romx_projection_keys");
        let mut table = transaction.open_table(definition).unwrap();
        let key = b"\x00\x08document\x00\x01a";
        assert!(table.remove(key.as_slice()).unwrap().is_some());
    }
    transaction.commit().unwrap();
    drop(database);
    let before = std::fs::read(directory.file()).unwrap();
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Corrupt)
    ));
    assert_eq!(std::fs::read(directory.file()).unwrap(), before);
}

#[test]
fn oversized_state_record_is_rejected_without_source_changes() {
    let directory = Directory::new();
    drop(CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap());
    let database = redb::Database::open(directory.file()).unwrap();
    let transaction = database.begin_write().unwrap();
    {
        let definition = redb::TableDefinition::<&[u8], &[u8]>::new("romx_projection_state");
        let mut table = transaction.open_table(definition).unwrap();
        table
            .insert(b"checkpoint".as_slice(), vec![0u8; 131073].as_slice())
            .unwrap();
    }
    transaction.commit().unwrap();
    drop(database);
    let before = std::fs::read(directory.file()).unwrap();
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::TooLarge)
    ));
    assert_eq!(std::fs::read(directory.file()).unwrap(), before);
}

#[test]
fn missing_application_marker_is_not_initialized_as_a_fresh_store() {
    let directory = Directory::new();
    drop(CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap());
    let database = redb::Database::open(directory.file()).unwrap();
    let transaction = database.begin_write().unwrap();
    {
        let definition = redb::TableDefinition::<&[u8], &[u8]>::new("romx_projection_meta");
        let mut table = transaction.open_table(definition).unwrap();
        table.remove(b"profile".as_slice()).unwrap();
    }
    transaction.commit().unwrap();
    drop(database);
    let before = std::fs::read(directory.file()).unwrap();
    assert!(matches!(
        CheckpointStore::open(&directory.file(), &profile()),
        Err(Error::Unsupported)
    ));
    assert_eq!(std::fs::read(directory.file()).unwrap(), before);
}

#[test]
fn hard_link_deployment_is_rejected_before_engine_open() {
    let directory = Directory::new();
    drop(CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap());
    let alias = directory.0.join("hard-link.redb");
    std::fs::hard_link(directory.file(), &alias).unwrap();
    let before = std::fs::read(directory.file()).unwrap();
    assert!(matches!(
        CheckpointStore::open(&alias, &profile()),
        Err(Error::Unsupported)
    ));
    assert_eq!(std::fs::read(directory.file()).unwrap(), before);
}
