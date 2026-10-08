//! Public admission failures before native storage work.
mod support;
use rom::{JournalCursor, Key};
use rom_projection_core::{
    Checkpoint, Error, OperationMetadata, PageIntent, ProjectionProfile, RemoteObservation,
};
use support::{cursor, initial};

#[test]
fn invalid_identity_and_zero_revision_are_rejected_before_store_access() {
    assert!(matches!(
        ProjectionProfile::new("", "qdrant", "v1", None),
        Err(Error::Invalid)
    ));
    assert!(matches!(
        ProjectionProfile::new(&"a".repeat(129), "qdrant", "v1", None),
        Err(Error::TooLarge)
    ));
    assert!(matches!(
        OperationMetadata::new(
            Key {
                kind: "document".into(),
                id: "a".into()
            },
            1,
            0,
            false,
            [1; 32]
        ),
        Err(Error::Invalid)
    ));
    assert!(matches!(
        OperationMetadata::new(
            Key {
                kind: "document".into(),
                id: "a".repeat(1025)
            },
            1,
            1,
            false,
            [1; 32]
        ),
        Err(Error::TooLarge)
    ));
}

#[test]
fn page_rejects_generation_gap_backward_progress_and_excess_work() {
    let changed = JournalCursor {
        generation: "different".into(),
        ..cursor(1)
    };
    assert!(matches!(
        PageIntent::new(&initial(), changed, vec![]),
        Err(Error::HistoryGap)
    ));
    let after = Checkpoint::new("physical-a", vec![cursor(5)]).unwrap();
    assert!(matches!(
        PageIntent::new(&after, cursor(4), vec![]),
        Err(Error::HistoryGap)
    ));
    let operations = (1..=65)
        .map(|position| {
            OperationMetadata::new(
                Key {
                    kind: "document".into(),
                    id: position.to_string(),
                },
                position,
                position,
                false,
                [1; 32],
            )
            .unwrap()
        })
        .collect();
    assert!(matches!(
        PageIntent::new(&initial(), cursor(65), operations),
        Err(Error::TooLarge)
    ));
}

#[test]
fn page_rejects_equal_revision_content_conflict_before_collapsing_keys() {
    let key = Key {
        kind: "document".into(),
        id: "a".into(),
    };
    let a = OperationMetadata::new(key.clone(), 1, 3, false, [1; 32]).unwrap();
    let b = OperationMetadata::new(key, 2, 3, false, [2; 32]).unwrap();
    assert!(matches!(
        PageIntent::new(&initial(), cursor(2), vec![a, b]),
        Err(Error::Conflict)
    ));
}

#[test]
fn remote_observation_rejects_oversized_target_before_owned_copy() {
    let operation = OperationMetadata::new(
        Key {
            kind: "document".into(),
            id: "a".into(),
        },
        1,
        1,
        false,
        [1; 32],
    )
    .unwrap();
    assert!(matches!(
        RemoteObservation::new(&support::profile(), &"x".repeat(129), operation),
        Err(Error::TooLarge)
    ));
}
