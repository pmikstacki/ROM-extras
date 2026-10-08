//! Real local-file intent and checkpoint durability.
mod support;

use rom::Key;
use rom_projection_core::{
    CheckpointStore, Error, OperationMetadata, PageIntent, RemoteObservation,
};
use support::{Directory, cursor, initial, profile};

fn operation(id: &str, position: u64, revision: u64, digest: [u8; 32]) -> OperationMetadata {
    OperationMetadata::new(
        Key {
            kind: "document".into(),
            id: id.into(),
        },
        position,
        revision,
        false,
        digest,
    )
    .unwrap()
}

#[test]
fn fresh_store_is_private_and_durable_after_full_close() {
    use std::os::unix::fs::PermissionsExt;
    let directory = Directory::new();
    let store = CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    assert_eq!(
        std::fs::metadata(directory.file())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    drop(store);
    let reopened = CheckpointStore::open(&directory.file(), &profile()).unwrap();
    assert_eq!(
        reopened.load().unwrap().checkpoint().cursor("document"),
        Some(&cursor(0))
    );
    assert!(reopened.load().unwrap().pending_page().is_none());
    assert!(matches!(
        CheckpointStore::create(&directory.file(), &profile(), &initial()),
        Err(Error::Busy)
    ));
}

#[test]
fn accepted_intent_survives_reopen_without_acknowledging_its_cursor() {
    let directory = Directory::new();
    let mut store = CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    let page = PageIntent::new(&initial(), cursor(3), vec![operation("a", 3, 7, [1; 32])]).unwrap();
    let token = store.prepare_page(&page).unwrap();
    assert_eq!(store.prepare_page(&page).unwrap(), token);
    drop(store);
    let reopened = CheckpointStore::open(&directory.file(), &profile()).unwrap();
    let state = reopened.load().unwrap();
    assert_eq!(state.checkpoint().cursor("document"), Some(&cursor(0)));
    assert_eq!(state.pending_page(), Some(&page));
    assert!(
        reopened
            .key_state(&Key {
                kind: "document".into(),
                id: "a".into()
            })
            .unwrap()
            .is_none()
    );
}

#[test]
fn complete_page_publishes_all_keys_and_cursor_together() {
    let directory = Directory::new();
    let mut store = CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    let a = operation("a", 1, 3, [1; 32]);
    let b = operation("b", 2, u64::MAX, [2; 32]);
    let page = PageIntent::new(&initial(), cursor(2), vec![a.clone(), b.clone()]).unwrap();
    store.prepare_page(&page).unwrap();
    let observations = vec![
        RemoteObservation::new(&profile(), "physical-a", a.clone()).unwrap(),
        RemoteObservation::new(&profile(), "physical-a", b.clone()).unwrap(),
    ];
    let reconciled = page.reconcile(&profile(), observations).unwrap();
    let token = store.complete_page(&page, &reconciled).unwrap();
    assert_eq!(store.complete_page(&page, &reconciled).unwrap(), token);
    drop(store);
    let reopened = CheckpointStore::open(&directory.file(), &profile()).unwrap();
    assert_eq!(
        reopened.load().unwrap().checkpoint().cursor("document"),
        Some(&cursor(2))
    );
    assert!(reopened.load().unwrap().pending_page().is_none());
    assert_eq!(reopened.key_state(a.key()).unwrap().unwrap().revision(), 3);
    assert_eq!(
        reopened.key_state(b.key()).unwrap().unwrap().revision(),
        u64::MAX
    );
    assert_eq!(
        reopened.key_state(b.key()).unwrap().unwrap().digest(),
        &[2; 32]
    );
}

#[test]
fn filtered_page_advances_without_inventing_key_state() {
    let directory = Directory::new();
    let mut store = CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    let page = PageIntent::new(&initial(), cursor(64), vec![]).unwrap();
    store.prepare_page(&page).unwrap();
    let reconciled = page.reconcile(&profile(), vec![]).unwrap();
    store.complete_page(&page, &reconciled).unwrap();
    drop(store);
    let reopened = CheckpointStore::open(&directory.file(), &profile()).unwrap();
    assert_eq!(
        reopened.load().unwrap().checkpoint().cursor("document"),
        Some(&cursor(64))
    );
}

#[test]
fn partial_or_conflicting_remote_observations_cannot_advance_cursor() {
    let directory = Directory::new();
    let mut store = CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    let expected = operation("a", 1, 7, [1; 32]);
    let page = PageIntent::new(&initial(), cursor(1), vec![expected]).unwrap();
    store.prepare_page(&page).unwrap();
    assert!(matches!(
        page.reconcile(&profile(), vec![]),
        Err(Error::Conflict)
    ));
    let wrong = operation("a", 1, 7, [2; 32]);
    assert!(matches!(
        page.reconcile(
            &profile(),
            vec![RemoteObservation::new(&profile(), "physical-a", wrong).unwrap()]
        ),
        Err(Error::Conflict)
    ));
    assert_eq!(
        store.load().unwrap().checkpoint().cursor("document"),
        Some(&cursor(0))
    );
    assert_eq!(store.load().unwrap().pending_page(), Some(&page));
}

#[test]
fn durable_key_state_rejects_stale_and_conflicting_equal_revision_after_reopen() {
    let directory = Directory::new();
    let mut store = CheckpointStore::create(&directory.file(), &profile(), &initial()).unwrap();
    let first = operation("a", 1, 7, [1; 32]);
    let page = PageIntent::new(&initial(), cursor(1), vec![first.clone()]).unwrap();
    store.prepare_page(&page).unwrap();
    let observed = page
        .reconcile(
            &profile(),
            vec![RemoteObservation::new(&profile(), "physical-a", first.clone()).unwrap()],
        )
        .unwrap();
    store.complete_page(&page, &observed).unwrap();
    drop(store);
    let mut store = CheckpointStore::open(&directory.file(), &profile()).unwrap();
    let before = store.load().unwrap();
    for (revision, digest) in [(6, [1; 32]), (7, [2; 32])] {
        let rejected = PageIntent::new(
            before.checkpoint(),
            cursor(2),
            vec![operation("a", 2, revision, digest)],
        )
        .unwrap();
        assert!(matches!(
            store.prepare_page(&rejected),
            Err(Error::Conflict)
        ));
        assert_eq!(store.load().unwrap(), before);
        assert_eq!(
            store.key_state(first.key()).unwrap().unwrap().digest(),
            &[1; 32]
        );
    }
}

#[test]
fn original_keys_with_ambiguous_separator_concatenation_remain_distinct() {
    let directory = Directory::new();
    let initial = rom_projection_core::Checkpoint::new(
        "physical-a",
        vec![
            rom::JournalCursor {
                kind: "document".into(),
                ..cursor(0)
            },
            rom::JournalCursor {
                kind: "document:a".into(),
                ..cursor(0)
            },
        ],
    )
    .unwrap();
    let keys = [
        Key {
            kind: "document".into(),
            id: "a:b".into(),
        },
        Key {
            kind: "document:a".into(),
            id: "b".into(),
        },
    ];
    let mut store = CheckpointStore::create(&directory.file(), &profile(), &initial).unwrap();
    for (index, key) in keys.iter().enumerate() {
        let operation =
            OperationMetadata::new(key.clone(), 1, 7 + index as u64, false, [index as u8; 32])
                .unwrap();
        let expected = store.load().unwrap().checkpoint().clone();
        let page = PageIntent::new(
            &expected,
            rom::JournalCursor {
                kind: key.kind.clone(),
                ..cursor(1)
            },
            vec![operation.clone()],
        )
        .unwrap();
        store.prepare_page(&page).unwrap();
        let observed = page
            .reconcile(
                &profile(),
                vec![RemoteObservation::new(&profile(), "physical-a", operation).unwrap()],
            )
            .unwrap();
        store.complete_page(&page, &observed).unwrap();
    }
    drop(store);
    let store = CheckpointStore::open(&directory.file(), &profile()).unwrap();
    assert_eq!(store.key_state(&keys[0]).unwrap().unwrap().revision(), 7);
    assert_eq!(store.key_state(&keys[1]).unwrap().unwrap().revision(), 8);
}
