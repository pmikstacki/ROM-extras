//! Portable ownership transitions, including actual staged rollback and unknown commit.
use rom_sql_core::{
    OwnerError, OwnerState, OwnerToken, OwnerTransaction, claim_owner, release_owner,
    takeover_owner, with_owner,
};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
struct Store {
    owner: OwnerState,
    value: u64,
}
struct Tx {
    durable: Rc<RefCell<Store>>,
    staged: Store,
    lose_ack: bool,
    fail_rollback: bool,
}
impl OwnerTransaction for Tx {
    fn lock_owner(&mut self) -> Result<OwnerState, OwnerError> {
        Ok(self.staged.owner)
    }
    fn write_owner(&mut self, state: OwnerState) -> Result<(), OwnerError> {
        self.staged.owner = state;
        Ok(())
    }
    fn commit(self) -> Result<(), OwnerError> {
        *self.durable.borrow_mut() = self.staged;
        if self.lose_ack {
            Err(OwnerError::Unknown)
        } else {
            Ok(())
        }
    }
    fn rollback(self) -> Result<(), OwnerError> {
        if self.fail_rollback {
            Err(OwnerError::Unknown)
        } else {
            Ok(())
        }
    }
}
fn token(byte: u8) -> OwnerToken {
    OwnerToken::new([byte; 32]).unwrap()
}
fn store() -> Rc<RefCell<Store>> {
    Rc::new(RefCell::new(Store {
        owner: OwnerState::new(0, None).unwrap(),
        value: 0,
    }))
}
fn tx(store: &Rc<RefCell<Store>>) -> Tx {
    Tx {
        durable: store.clone(),
        staged: store.borrow().clone(),
        lose_ack: false,
        fail_rollback: false,
    }
}
#[test]
fn stale_connection_cannot_write_or_release_after_takeover() {
    let db = store();
    let first = claim_owner(tx(&db), token(1)).unwrap();
    assert_eq!(claim_owner(tx(&db), token(2)), Err(OwnerError::Busy));
    with_owner(tx(&db), first, |tx| {
        tx.staged.value = 17;
        Ok(())
    })
    .unwrap();
    let observed = db.borrow().owner;
    let second = takeover_owner(tx(&db), observed, token(2)).unwrap();
    assert_eq!(second.generation(), 2);
    let mut entered = false;
    assert_eq!(
        with_owner(tx(&db), first, |_| {
            entered = true;
            Ok(())
        }),
        Err(OwnerError::Stale)
    );
    assert!(!entered);
    assert_eq!(release_owner(tx(&db), first), Err(OwnerError::Stale));
    assert_eq!(
        takeover_owner(tx(&db), observed, token(3)),
        Err(OwnerError::Stale)
    );
    assert_eq!(db.borrow().value, 17);
    release_owner(tx(&db), second).unwrap();
    assert_eq!(claim_owner(tx(&db), token(1)).unwrap().generation(), 3);
}
#[test]
fn failed_native_work_rolls_back_every_staged_write() {
    let db = store();
    let owner = claim_owner(tx(&db), token(1)).unwrap();
    let result: Result<(), _> = with_owner(tx(&db), owner, |tx| {
        tx.staged.value = 99;
        Err(OwnerError::Unavailable)
    });
    assert_eq!(result, Err(OwnerError::Unavailable));
    assert_eq!(db.borrow().value, 0);
    let mut uncertain = tx(&db);
    uncertain.fail_rollback = true;
    let result: Result<(), _> = with_owner(uncertain, owner, |_| Err(OwnerError::Unavailable));
    assert_eq!(result, Err(OwnerError::Unknown));
}
#[test]
fn unknown_claim_does_not_return_usable_ownership() {
    let db = store();
    let mut uncertain = tx(&db);
    uncertain.lose_ack = true;
    assert_eq!(claim_owner(uncertain, token(1)), Err(OwnerError::Unknown));
    assert_eq!(db.borrow().owner.generation(), 1);
    assert_eq!(claim_owner(tx(&db), token(2)), Err(OwnerError::Busy));
    let observed = db.borrow().owner;
    assert_eq!(
        takeover_owner(tx(&db), observed, token(2))
            .unwrap()
            .generation(),
        2
    );
}
#[test]
fn generation_exhaustion_never_resets_or_releases_owner() {
    let db = store();
    db.borrow_mut().owner = OwnerState::new(i64::MAX as u64, Some(token(1))).unwrap();
    let expected = db.borrow().owner;
    assert_eq!(
        takeover_owner(tx(&db), expected, token(2)),
        Err(OwnerError::Exhausted)
    );
    assert_eq!(db.borrow().owner, expected);
    db.borrow_mut().owner = OwnerState::new(i64::MAX as u64, None).unwrap();
    assert_eq!(claim_owner(tx(&db), token(2)), Err(OwnerError::Exhausted));
}
#[test]
fn bounds_and_debug_do_not_disclose_tokens() {
    assert_eq!(OwnerToken::new([0; 32]), Err(OwnerError::Invalid));
    assert_eq!(OwnerState::new(0, Some(token(1))), Err(OwnerError::Invalid));
    assert_eq!(
        OwnerState::new(i64::MAX as u64 + 1, None),
        Err(OwnerError::Invalid)
    );
    assert_eq!(format!("{:?}", token(1)), "OwnerToken { redacted }");
    assert!(!format!("{:?}", OwnerState::new(1, Some(token(1))).unwrap()).contains("[1,"));
}
