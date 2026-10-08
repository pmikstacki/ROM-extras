//! Exclusive durable local intent and checkpoint transactions.
use crate::{Error, Result, codec, metadata::*, preflight};
use redb::{Durability, ReadableDatabase, ReadableTable};
use rom_backup::{NativeAccess, NativeOwnership};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

#[cfg(test)]
#[path = "checkpoint_faults.rs"]
mod faults;

pub(crate) const META: redb::TableDefinition<&[u8], &[u8]> =
    redb::TableDefinition::new("romx_projection_meta");
pub(crate) const STATE: redb::TableDefinition<&[u8], &[u8]> =
    redb::TableDefinition::new("romx_projection_state");
pub(crate) const KEYS: redb::TableDefinition<&[u8], &[u8]> =
    redb::TableDefinition::new("romx_projection_keys");

/// One cooperating local Linux owner. The engine closes before its reservation.
pub struct CheckpointStore {
    engine: Option<redb::Database>,
    owner: NativeOwnership,
    profile: ProjectionProfile,
}
impl CheckpointStore {
    /// Create absent private storage; synchronize initialization and its parent directory.
    pub fn create(path: &Path, profile: &ProjectionProfile, initial: &Checkpoint) -> Result<Self> {
        if initial.sequence != 0 {
            return Err(Error::Invalid);
        }
        let owner = NativeOwnership::acquire(path, NativeAccess::Fresh).map_err(ownership_error)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(owner.path())
            .map_err(|_| Error::Storage)?;
        let engine = preflight::builder()
            .create_file(file)
            .map_err(preflight::database_error)?;
        let mut store = Self {
            engine: Some(engine),
            owner,
            profile: profile.clone(),
        };
        let snapshot = StoreSnapshot {
            checkpoint: initial.clone(),
            pending: None,
            completed: None,
            completed_page: None,
        };
        store.write(None, |transaction| {
            transaction
                .open_table(META)
                .map_err(|_| Error::Storage)?
                .insert(b"profile".as_slice(), codec::profile(profile).as_slice())
                .map_err(|_| Error::Storage)?;
            transaction
                .open_table(STATE)
                .map_err(|_| Error::Storage)?
                .insert(b"checkpoint".as_slice(), codec::state(&snapshot).as_slice())
                .map_err(|_| Error::Storage)?;
            transaction.open_table(KEYS).map_err(|_| Error::Storage)?;
            Ok(())
        })?;
        File::open(store.owner.path().parent().ok_or(Error::Unsupported)?)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| Error::UnknownInitialization)?;
        Ok(store)
    }
    /// Admit existing application format before repairing original native bytes.
    pub fn open(path: &Path, profile: &ProjectionProfile) -> Result<Self> {
        Self::open_cancellable(path, profile, &crate::Cancellation::new())
    }
    /// Admit existing storage with cooperative startup cancellation.
    pub fn open_cancellable(
        path: &Path,
        profile: &ProjectionProfile,
        cancel: &crate::Cancellation,
    ) -> Result<Self> {
        cancel.check()?;
        let owner =
            NativeOwnership::acquire(path, NativeAccess::Existing).map_err(ownership_error)?;
        let engine = admitted_engine(owner.path(), profile, cancel)?;
        Ok(Self {
            engine: Some(engine),
            owner,
            profile: profile.clone(),
        })
    }
    /// Close a retired handle and recover under its retained ownership reservation.
    pub fn reopen(&mut self) -> Result<()> {
        self.reopen_cancellable(&crate::Cancellation::new())
    }
    /// Reopen with cancellation. Once admission starts, cancellation leaves the engine retired.
    /// Ownership remains held until this store closes; native repair may already have changed bytes.
    pub fn reopen_cancellable(&mut self, cancel: &crate::Cancellation) -> Result<()> {
        cancel.check()?;
        self.engine.take();
        self.engine = Some(admitted_engine(self.owner.path(), &self.profile, cancel)?);
        Ok(())
    }
    fn engine(&self) -> Result<&redb::Database> {
        self.engine.as_ref().ok_or(Error::Storage)
    }
    /// Load bounded durable control state without exposing a native transaction.
    pub fn load(&self) -> Result<StoreSnapshot> {
        preflight::snapshot(&self.engine()?.begin_read().map_err(|_| Error::Storage)?)
    }
    /// Read one exact original key's last durable acknowledged representation.
    pub fn key_state(&self, key: &rom::Key) -> Result<Option<KeyState>> {
        crate::limits::key(key)?;
        let transaction = self.engine()?.begin_read().map_err(|_| Error::Storage)?;
        let table = transaction.open_table(KEYS).map_err(|_| Error::Corrupt)?;
        table
            .get(codec::key(key).as_slice())
            .map_err(|_| Error::Storage)?
            .map(|value| codec::decode_key_state(value.value()))
            .transpose()
    }
    /// Persist intent with cursor compare-and-set before any remote dispatch.
    pub fn prepare_page(&mut self, page: &PageIntent) -> Result<TransactionId> {
        let state = self.load()?;
        let fingerprint = codec::page_digest(&self.profile, page);
        if let Some(pending) = &state.pending {
            if pending == page {
                return Ok(prepare_token(&self.profile, &state, page));
            }
            return Err(Error::Conflict);
        }
        if state.completed_page.as_ref() == Some(page)
            && let Some((prepare, _)) = &state.completed
        {
            return Ok(prepare.clone());
        }
        if state.checkpoint != page.expected {
            return Err(Error::Conflict);
        }
        if page.expected.cursor(&page.next.kind) == Some(&page.next) {
            return Err(Error::Invalid);
        }
        let token = TransactionId {
            sequence: state
                .checkpoint
                .sequence
                .checked_add(1)
                .ok_or(Error::TooLarge)?,
            transition: 1,
            fingerprint,
            previous: codec::control_digest(&self.profile, &state),
        };
        let mut next = state.clone();
        next.checkpoint.sequence = token.sequence;
        next.pending = Some(page.clone());
        self.write(Some(&token), |transaction| {
            compare_control(transaction, &state)?;
            let keys = transaction.open_table(KEYS).map_err(|_| Error::Corrupt)?;
            for operation in &page.operations {
                if let Some(value) = keys
                    .get(codec::key(&operation.key).as_slice())
                    .map_err(|_| Error::Storage)?
                {
                    monotonic(
                        &codec::decode_key_state(value.value())?,
                        &operation.key_state(),
                    )?;
                }
            }
            let mut control = transaction.open_table(STATE).map_err(|_| Error::Corrupt)?;
            control
                .insert(b"checkpoint".as_slice(), codec::state(&next).as_slice())
                .map_err(|_| Error::Storage)?;
            control
                .insert(b"pending".as_slice(), codec::page(page).as_slice())
                .map_err(|_| Error::Storage)?;
            Ok(())
        })?;
        Ok(token)
    }
    /// Atomically acknowledge exact reconciled key states and advance the inspected cursor.
    pub fn complete_page(
        &mut self,
        page: &PageIntent,
        observed: &ReconciledPage,
    ) -> Result<TransactionId> {
        let state = self.load()?;
        let fingerprint = codec::page_digest(&self.profile, page);
        if observed.fingerprint != fingerprint {
            return Err(Error::Conflict);
        }
        if state.pending.is_none() {
            if let Some((_, token)) = &state.completed
                && token.fingerprint == fingerprint
            {
                return Ok(token.clone());
            }
            return Err(Error::Conflict);
        }
        if state.pending.as_ref() != Some(page) {
            return Err(Error::Conflict);
        }
        let prepare = prepare_token(&self.profile, &state, page);
        let token = TransactionId {
            sequence: state
                .checkpoint
                .sequence
                .checked_add(1)
                .ok_or(Error::TooLarge)?,
            transition: 2,
            fingerprint,
            previous: codec::control_digest(&self.profile, &state),
        };
        let mut next = state.clone();
        next.checkpoint.sequence = token.sequence;
        *next
            .checkpoint
            .cursors
            .iter_mut()
            .find(|cursor| cursor.kind == page.next.kind)
            .ok_or(Error::Corrupt)? = page.next.clone();
        next.pending = None;
        next.completed = Some((prepare, token.clone()));
        next.completed_page = Some(page.clone());
        self.write(Some(&token), |transaction| {
            compare_control(transaction, &state)?;
            let mut keys = transaction.open_table(KEYS).map_err(|_| Error::Corrupt)?;
            for operation in &page.operations {
                let key = codec::key(&operation.key);
                let value = operation.key_state();
                if let Some(prior) = keys.get(key.as_slice()).map_err(|_| Error::Storage)? {
                    monotonic(&codec::decode_key_state(prior.value())?, &value)?;
                }
                keys.insert(key.as_slice(), codec::key_state(&value).as_slice())
                    .map_err(|_| Error::Storage)?;
            }
            let mut control = transaction.open_table(STATE).map_err(|_| Error::Corrupt)?;
            control
                .insert(b"checkpoint".as_slice(), codec::state(&next).as_slice())
                .map_err(|_| Error::Storage)?;
            control
                .remove(b"pending".as_slice())
                .map_err(|_| Error::Storage)?;
            control
                .insert(b"completed_page".as_slice(), codec::page(page).as_slice())
                .map_err(|_| Error::Storage)?;
            Ok(())
        })?;
        Ok(token)
    }
    /// Classify only a retained exact applied transition or its proven previous state.
    pub fn reconcile(&self, transaction: &TransactionId) -> Result<CommitStatus> {
        let state = self.load()?;
        if let Some((prepare, complete)) = &state.completed
            && (transaction == prepare || transaction == complete)
        {
            return Ok(CommitStatus::Applied);
        }
        if let Some(page) = &state.pending
            && *transaction == prepare_token(&self.profile, &state, page)
        {
            return Ok(CommitStatus::Applied);
        }
        if state.checkpoint.sequence.checked_add(1) == Some(transaction.sequence)
            && codec::control_digest(&self.profile, &state) == transaction.previous
        {
            match (&state.pending, transaction.transition) {
                (None, 1) => return Ok(CommitStatus::Absent),
                (Some(page), 2)
                    if codec::page_digest(&self.profile, page) == transaction.fingerprint =>
                {
                    return Ok(CommitStatus::Absent);
                }
                _ => {}
            }
        }
        Err(Error::Conflict)
    }
    fn write(
        &mut self,
        token: Option<&TransactionId>,
        operation: impl FnOnce(&redb::WriteTransaction) -> Result<()>,
    ) -> Result<()> {
        let mut transaction = self.engine()?.begin_write().map_err(|_| Error::Storage)?;
        transaction
            .set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        if let Err(error) = operation(&transaction) {
            if transaction.abort().is_err() {
                self.engine.take();
                return Err(Error::Storage);
            }
            return Err(error);
        }
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| transaction.commit())) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(redb::CommitError::TransactionPoisoned)) => {
                self.engine.take();
                Err(Error::Corrupt)
            }
            _ => {
                self.engine.take();
                Err(token
                    .map(|transaction| Error::Unknown {
                        transaction: transaction.clone(),
                    })
                    .unwrap_or(Error::UnknownInitialization))
            }
        }
    }
}

fn ownership_error(error: rom::Error) -> Error {
    match error {
        rom::Error::Conflict => Error::Busy,
        rom::Error::Unsupported(_) => Error::Unsupported,
        _ => Error::Storage,
    }
}
fn admitted_engine(
    path: &Path,
    profile: &ProjectionProfile,
    cancel: &crate::Cancellation,
) -> Result<redb::Database> {
    preflight::admit(path, profile, cancel)?;
    cancel.check()?;
    let engine = preflight::cancellable_builder(cancel)
        .open(path)
        .map_err(|error| preflight::cancellable_database_error(error, cancel))?;
    preflight::validate_cancellable(
        &engine.begin_read().map_err(|_| Error::Storage)?,
        profile,
        &mut || cancel.check(),
    )?;
    cancel.check()?;
    Ok(engine)
}
fn prepare_token(
    profile: &ProjectionProfile,
    state: &StoreSnapshot,
    page: &PageIntent,
) -> TransactionId {
    let previous = StoreSnapshot {
        checkpoint: page.expected.clone(),
        pending: None,
        completed: state.completed.clone(),
        completed_page: state.completed_page.clone(),
    };
    TransactionId {
        sequence: state.checkpoint.sequence,
        transition: 1,
        fingerprint: codec::page_digest(profile, page),
        previous: codec::control_digest(profile, &previous),
    }
}
fn monotonic(prior: &KeyState, next: &KeyState) -> Result<()> {
    if prior.revision > next.revision || (prior.revision == next.revision && prior != next) {
        return Err(Error::Conflict);
    }
    Ok(())
}
fn compare_control(transaction: &redb::WriteTransaction, state: &StoreSnapshot) -> Result<()> {
    let table = transaction.open_table(STATE).map_err(|_| Error::Corrupt)?;
    if table
        .get(b"checkpoint".as_slice())
        .map_err(|_| Error::Storage)?
        .is_none_or(|value| value.value() != codec::state(state))
    {
        return Err(Error::Conflict);
    }
    let expected = state.pending.as_ref().map(codec::page);
    let actual = table
        .get(b"pending".as_slice())
        .map_err(|_| Error::Storage)?;
    if actual.as_ref().map(|value| value.value()) != expected.as_deref() {
        return Err(Error::Conflict);
    }
    let expected_completed = state.completed_page.as_ref().map(codec::page);
    let actual_completed = table
        .get(b"completed_page".as_slice())
        .map_err(|_| Error::Storage)?;
    if actual_completed.as_ref().map(|value| value.value()) != expected_completed.as_deref() {
        return Err(Error::Conflict);
    }
    Ok(())
}
