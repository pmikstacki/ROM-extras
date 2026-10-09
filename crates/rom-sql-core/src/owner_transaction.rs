//! Shared ownership arbitration over an already-opened native transaction.
use crate::{OwnerError, OwnerState, OwnerToken, Ownership};
/// Driver port for one native transaction with control-row arbitration.
///
/// `lock_owner` must lock the singleton exclusively, validate immutable format/store identity,
/// and return bounded exact state. Retain the lock until commit or rollback.
/// Every protected write and owner transition uses this same row and native transaction.
/// No normal open or lock operation may provision, migrate or reset an unknown format.
/// Drivers impose connect, statement and lock deadlines and roll back on Drop.
/// Commit errors must be Unknown unless the driver proves rollback. Uncertain rollback is Unknown.
pub trait OwnerTransaction: Sized {
    /// Lock and read validated authoritative ownership.
    fn lock_owner(&mut self) -> Result<OwnerState, OwnerError>;
    /// Replace ownership while retaining its lock. Exactly one row must be updated.
    fn write_owner(&mut self, state: OwnerState) -> Result<(), OwnerError>;
    /// Commit all staged writes before reporting success; consume the transaction.
    fn commit(self) -> Result<(), OwnerError>;
    /// Confirm complete rollback; consume the transaction. Drop remains a fallback.
    fn rollback(self) -> Result<(), OwnerError>;
}
fn finish<T: OwnerTransaction, R>(tx: T, result: Result<R, OwnerError>) -> Result<R, OwnerError> {
    match result {
        Ok(value) => {
            tx.commit()?;
            Ok(value)
        }
        Err(error) => match tx.rollback() {
            Ok(()) => Err(error),
            Err(_) => Err(OwnerError::Unknown),
        },
    }
}
/// Claim an idle control row. Busy or exhausted states remain unchanged.
/// No usable ownership is returned after an unknown commit.
pub fn claim_owner<T: OwnerTransaction>(
    mut tx: T,
    token: OwnerToken,
) -> Result<Ownership, OwnerError> {
    let result = (|| {
        let current = tx.lock_owner()?;
        if current.token().is_some() {
            return Err(OwnerError::Busy);
        }
        let next = current.advance(token)?;
        tx.write_owner(next.state())?;
        Ok(next)
    })();
    finish(tx, result)
}
/// Explicit host-authorized takeover of an exact observed state. No automatic liveness inference.
pub fn takeover_owner<T: OwnerTransaction>(
    mut tx: T,
    expected: OwnerState,
    token: OwnerToken,
) -> Result<Ownership, OwnerError> {
    let result = (|| {
        if tx.lock_owner()? != expected {
            return Err(OwnerError::Stale);
        }
        let next = expected.advance(token)?;
        tx.write_owner(next.state())?;
        Ok(next)
    })();
    finish(tx, result)
}
/// Clear only the exact acknowledged owner. Retain the monotonic generation.
pub fn release_owner<T: OwnerTransaction>(mut tx: T, owner: Ownership) -> Result<(), OwnerError> {
    let result = (|| {
        if tx.lock_owner()? != owner.state() {
            return Err(OwnerError::Stale);
        }
        tx.write_owner(OwnerState::new(owner.generation(), None)?)
    })();
    finish(tx, result)
}
/// Verify ownership and commit prepared native persistence work under the same control-row lock.
///
/// The callback is a driver boundary, not a domain action hook. Do not execute application
/// policies, author codecs, external effects or network requests while holding these locks.
/// Success returns the callback value only after commit. Errors roll back the entire transaction.
pub fn with_owner<T: OwnerTransaction, R>(
    mut tx: T,
    owner: Ownership,
    write: impl FnOnce(&mut T) -> Result<R, OwnerError>,
) -> Result<R, OwnerError> {
    let result = (|| {
        if tx.lock_owner()? != owner.state() {
            return Err(OwnerError::Stale);
        }
        write(&mut tx)
    })();
    finish(tx, result)
}
