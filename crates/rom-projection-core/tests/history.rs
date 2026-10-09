//! Exact interval reconstruction, distinct from native feed/provider qualification.
mod support;
use rom::{JournalBatch, JournalView, Key, ProjectedView};
use rom_projection_core::{Cancellation, Error, OperationMetadata, PageIntent, PendingHistory};
use support::{cursor, initial};
fn event(position: u64, revision: u64, deleted: bool) -> JournalView {
    JournalView {
        position,
        view: ProjectedView {
            key: Key {
                kind: "document".into(),
                id: "private-key".into(),
            },
            revision,
            value: (!deleted).then(Default::default),
        },
    }
}
fn map(view: &JournalView) -> rom_projection_core::Result<OperationMetadata> {
    OperationMetadata::new(
        view.view.key.clone(),
        view.position,
        view.view.revision,
        view.view.value.is_none(),
        [view.view.revision as u8; 32],
    )
}
fn intent(events: &[JournalView], end: u64) -> PageIntent {
    PageIntent::new(
        &initial(),
        cursor(end),
        events.iter().map(|e| map(e).unwrap()).collect(),
    )
    .unwrap()
}
fn batch(events: Vec<JournalView>, end: u64) -> JournalBatch {
    JournalBatch {
        events,
        cursor: cursor(end),
    }
}
#[test]
fn replay_overrun_maps_only_old_interval_and_returns_exact_old_intent() {
    let old = intent(&[event(2, 1, false), event(4, 2, true)], 5);
    let mut replay = PendingHistory::new(old.clone()).unwrap();
    assert_eq!(replay.cursor(), &cursor(0));
    replay
        .push(
            &batch(vec![event(2, 1, false)], 3),
            &Cancellation::new(),
            map,
        )
        .unwrap();
    assert_eq!(replay.cursor(), &cursor(3));
    let mut mapped = 0;
    replay
        .push(
            &batch(vec![event(4, 2, true), event(8, 3, false)], 10),
            &Cancellation::new(),
            |e| {
                mapped += 1;
                map(e)
            },
        )
        .unwrap();
    assert_eq!(mapped, 1);
    assert_eq!(replay.cursor(), &cursor(5));
    assert_eq!(replay.finish().unwrap(), old);
}
#[test]
fn filtered_empty_history_advances_only_after_inspected_coverage() {
    let old = intent(&[], 6);
    let mut replay = PendingHistory::new(old.clone()).unwrap();
    replay
        .push(&batch(vec![], 3), &Cancellation::new(), map)
        .unwrap();
    assert_eq!(replay.finish(), Err(Error::HistoryGap));
    let mut replay = PendingHistory::new(old.clone()).unwrap();
    replay
        .push(&batch(vec![], 8), &Cancellation::new(), map)
        .unwrap();
    assert_eq!(replay.finish().unwrap(), old);
}
#[test]
fn changed_disclosure_and_changed_mapping_require_rebuild() {
    let old = intent(&[event(2, 1, false)], 3);
    let mut replay = PendingHistory::new(old.clone()).unwrap();
    replay
        .push(&batch(vec![], 3), &Cancellation::new(), map)
        .unwrap();
    assert_eq!(replay.finish(), Err(Error::RebuildRequired));
    let mut replay = PendingHistory::new(old).unwrap();
    replay
        .push(
            &batch(vec![event(2, 1, false)], 3),
            &Cancellation::new(),
            |e| {
                OperationMetadata::new(
                    e.view.key.clone(),
                    e.position,
                    e.view.revision,
                    false,
                    [99; 32],
                )
            },
        )
        .unwrap();
    assert_eq!(replay.finish(), Err(Error::RebuildRequired));
}
#[test]
fn malformed_overrun_is_rejected_before_any_mapping_and_poisoned() {
    let mut replay = PendingHistory::new(intent(&[event(2, 1, false)], 3)).unwrap();
    let mut calls = 0;
    assert_eq!(
        replay.push(
            &batch(
                vec![event(2, 1, false), event(6, 2, false), event(5, 3, false)],
                8
            ),
            &Cancellation::new(),
            |e| {
                calls += 1;
                map(e)
            }
        ),
        Err(Error::HistoryGap)
    );
    assert_eq!(calls, 0);
    assert_eq!(
        replay.push(
            &batch(vec![event(2, 1, false)], 3),
            &Cancellation::new(),
            map
        ),
        Err(Error::HistoryGap)
    );
    assert_eq!(replay.finish(), Err(Error::HistoryGap));
}
#[test]
fn cancellation_preserves_cursor_and_accepts_a_later_uncancelled_attempt() {
    let old = intent(&[], 3);
    let mut replay = PendingHistory::new(old.clone()).unwrap();
    let cancel = Cancellation::new();
    cancel.cancel();
    assert_eq!(
        replay.push(&batch(vec![], 3), &cancel, map),
        Err(Error::Cancelled)
    );
    assert_eq!(replay.cursor(), &cursor(0));
    replay
        .push(&batch(vec![], 3), &Cancellation::new(), map)
        .unwrap();
    assert_eq!(replay.finish().unwrap(), old);
}
#[test]
fn mapper_cannot_substitute_original_identity_or_tombstone() {
    let mut replay = PendingHistory::new(intent(&[event(2, 1, true)], 3)).unwrap();
    assert_eq!(
        replay.push(
            &batch(vec![event(2, 1, true)], 3),
            &Cancellation::new(),
            |e| {
                OperationMetadata::new(
                    e.view.key.clone(),
                    e.position,
                    e.view.revision,
                    false,
                    [1; 32],
                )
            }
        ),
        Err(Error::RebuildRequired)
    );
    assert_eq!(replay.finish(), Err(Error::RebuildRequired));
}
#[test]
fn stalled_generation_change_and_work_exhaustion_fail_closed() {
    for mut b in [batch(vec![], 0), batch(vec![], 5000), batch(vec![], 2)] {
        if b.cursor.position == 2 {
            b.cursor.generation = "different".into();
        }
        let expected = if b.cursor.position == 5000 {
            Error::TooLarge
        } else {
            Error::HistoryGap
        };
        let mut replay = PendingHistory::new(intent(&[], 10)).unwrap();
        assert_eq!(
            replay.push(&b, &Cancellation::new(), map),
            Err(expected.clone())
        );
        assert_eq!(replay.finish(), Err(expected));
    }
}

#[test]
fn mapper_panic_cannot_leave_a_reusable_partial_reconstruction() {
    let mut replay = PendingHistory::new(intent(&[event(2, 1, false)], 3)).unwrap();
    let b = batch(vec![event(2, 1, false)], 3);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            replay.push(&b, &Cancellation::new(), |_| {
                panic!("controlled mapper unwind")
            })
        }))
        .is_err()
    );
    assert_eq!(replay.cursor(), &cursor(0));
    assert_eq!(
        replay.push(&b, &Cancellation::new(), map),
        Err(Error::RebuildRequired)
    );
    assert_eq!(replay.finish(), Err(Error::RebuildRequired));
}
#[test]
fn revision_regression_and_equal_revision_mismatch_across_batches_fail() {
    for (first, second) in [(2, 1), (2, 2)] {
        let mut replay = PendingHistory::new(intent(&[event(3, 2, false)], 4)).unwrap();
        replay
            .push(
                &batch(vec![event(1, first, false)], 2),
                &Cancellation::new(),
                map,
            )
            .unwrap();
        assert_eq!(
            replay.push(
                &batch(vec![event(3, second, false)], 4),
                &Cancellation::new(),
                |e| {
                    OperationMetadata::new(
                        e.view.key.clone(),
                        e.position,
                        e.view.revision,
                        false,
                        [99; 32],
                    )
                }
            ),
            Err(Error::RebuildRequired)
        );
    }
}
#[test]
fn fetch_budget_is_finite_and_empty_noop_requires_no_fetch() {
    let mut replay = PendingHistory::new(intent(&[], 100)).unwrap();
    for n in 1..=64 {
        replay
            .push(&batch(vec![], n), &Cancellation::new(), map)
            .unwrap();
    }
    assert_eq!(
        replay.push(&batch(vec![], 65), &Cancellation::new(), map),
        Err(Error::TooLarge)
    );
    assert_eq!(replay.finish(), Err(Error::TooLarge));
    let old = intent(&[], 0);
    assert_eq!(
        PendingHistory::new(old.clone()).unwrap().finish().unwrap(),
        old
    );
}
#[test]
fn reconstruction_does_not_publish_or_replace_the_actual_durable_pending_intent() {
    use rom_projection_core::CheckpointStore;
    let dir = support::Directory::new();
    let profile = support::profile();
    let old = intent(&[event(2, 1, true)], 3);
    let mut store = CheckpointStore::create(&dir.file(), &profile, &initial()).unwrap();
    store.prepare_page(&old).unwrap();
    drop(store);
    let store = CheckpointStore::open(&dir.file(), &profile).unwrap();
    let pending = store.load().unwrap().pending_page().unwrap().clone();
    let mut replay = PendingHistory::new(pending).unwrap();
    replay
        .push(&batch(vec![], 3), &Cancellation::new(), map)
        .unwrap();
    assert_eq!(replay.finish(), Err(Error::RebuildRequired));
    assert_eq!(store.load().unwrap().pending_page(), Some(&old));
    assert_eq!(
        store.load().unwrap().checkpoint().cursor("document"),
        Some(&cursor(0))
    );
    assert_eq!(store.key_state(old.operations()[0].key()).unwrap(), None);
}
