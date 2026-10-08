//! Boundaries requiring an internal near-exhausted durable sequence fixture.
use super::*;
#[test]
fn sequence_exhaustion_is_rejected_before_preparing_an_unfinishable_page() {
    let mut checkpoint = Checkpoint::new(
        "physical-a",
        vec![JournalCursor {
            generation: "journal-a".into(),
            kind: "document".into(),
            position: 0,
        }],
    )
    .unwrap();
    for sequence in [u64::MAX - 1, u64::MAX] {
        checkpoint.sequence = sequence;
        let next = JournalCursor {
            generation: "journal-a".into(),
            kind: "document".into(),
            position: 1,
        };
        assert!(matches!(
            PageIntent::new(&checkpoint, next, vec![]),
            Err(Error::TooLarge)
        ));
    }
}
