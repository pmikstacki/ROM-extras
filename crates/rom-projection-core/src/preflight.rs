//! Source-preserving format admission and bounded private-copy native recovery.
use crate::{
    Error, Result,
    checkpoint::{KEYS, META, STATE},
    codec, limits,
    metadata::*,
};
use redb::{ReadableDatabase, ReadableTable, TableHandle};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_COPY: AtomicU64 = AtomicU64::new(0);
struct CopyPath(PathBuf);
impl Drop for CopyPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub(crate) fn builder() -> redb::Builder {
    let mut builder = redb::Database::builder();
    builder.set_cache_size(limits::CACHE);
    builder
}

pub(crate) fn database_error(error: redb::DatabaseError) -> Error {
    match error {
        redb::DatabaseError::DatabaseAlreadyOpen => Error::Busy,
        redb::DatabaseError::UpgradeRequired(_) => Error::Unsupported,
        _ => Error::Storage,
    }
}

pub(crate) fn cancellable_builder(cancel: &crate::Cancellation) -> redb::Builder {
    let mut builder = builder();
    let cancel = cancel.clone();
    builder.set_repair_callback(move |session| {
        if cancel.is_cancelled() {
            session.abort();
        }
    });
    builder
}
pub(crate) fn cancellable_database_error(
    error: redb::DatabaseError,
    cancel: &crate::Cancellation,
) -> Error {
    if matches!(error, redb::DatabaseError::RepairAborted) && cancel.is_cancelled() {
        Error::Cancelled
    } else {
        database_error(error)
    }
}
pub(crate) fn admit(
    path: &Path,
    profile: &ProjectionProfile,
    cancel: &crate::Cancellation,
) -> Result<()> {
    cancel.check()?;
    match builder().open_read_only(path) {
        Ok(database) => validate_cancellable(
            &database.begin_read().map_err(|_| Error::Storage)?,
            profile,
            &mut || cancel.check(),
        ),
        Err(redb::DatabaseError::RepairAborted) => validate_repaired_copy(path, profile, cancel),
        Err(error) => Err(database_error(error)),
    }
}

fn validate_repaired_copy(
    path: &Path,
    profile: &ProjectionProfile,
    cancel: &crate::Cancellation,
) -> Result<()> {
    cancel.check()?;
    let mut source = File::open(path).map_err(|_| Error::Storage)?;
    let before = source.metadata().map_err(|_| Error::Storage)?;
    if before.len() > limits::RECOVERY_FILE {
        return Err(Error::TooLarge);
    }
    let parent = path.parent().ok_or(Error::Unsupported)?;
    let target = parent.join(format!(
        ".romx-projection-preflight-{}-{}",
        std::process::id(),
        NEXT_COPY.fetch_add(1, Ordering::Relaxed)
    ));
    let mut output = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&target)
        .map_err(|_| Error::Storage)?;
    let temporary = CopyPath(target);
    let copied = copy_contents(&mut source, &mut output, before.len(), &mut || {
        cancel.check()
    })?;
    cancel.check()?;
    output.sync_all().map_err(|_| Error::Storage)?;
    let after = source.metadata().map_err(|_| Error::Storage)?;
    let current = std::fs::metadata(path).map_err(|_| Error::Storage)?;
    let identity = |metadata: &std::fs::Metadata| {
        (
            metadata.dev(),
            metadata.ino(),
            metadata.len(),
            metadata.mtime(),
            metadata.mtime_nsec(),
        )
    };
    if copied != before.len()
        || identity(&before) != identity(&after)
        || identity(&before) != identity(&current)
    {
        return Err(Error::Conflict);
    }
    let result = {
        cancel.check()?;
        let database = cancellable_builder(cancel)
            .create_file(output)
            .map_err(|error| cancellable_database_error(error, cancel))?;
        validate_cancellable(
            &database.begin_read().map_err(|_| Error::Storage)?,
            profile,
            &mut || cancel.check(),
        )
    };
    drop(temporary);
    result
}

pub(crate) fn snapshot(transaction: &redb::ReadTransaction) -> Result<StoreSnapshot> {
    let table = transaction
        .open_table(STATE)
        .map_err(|_| Error::Unsupported)?;
    let guard = table
        .get(b"checkpoint".as_slice())
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Corrupt)?;
    let mut value = codec::decode_state(guard.value())?;
    if let Some(pending) = table
        .get(b"pending".as_slice())
        .map_err(|_| Error::Storage)?
    {
        value.pending = Some(codec::decode_page(pending.value())?);
    }
    if let Some(completed) = table
        .get(b"completed_page".as_slice())
        .map_err(|_| Error::Storage)?
    {
        value.completed_page = Some(codec::decode_page(completed.value())?);
    }
    Ok(value)
}

pub(crate) fn validate_cancellable(
    transaction: &redb::ReadTransaction,
    profile: &ProjectionProfile,
    check: &mut impl FnMut() -> Result<()>,
) -> Result<()> {
    check()?;
    let mut tables = transaction.list_tables().map_err(|_| Error::Storage)?;
    let mut seen = 0u8;
    for _ in 0..4 {
        let Some(table) = tables.next() else {
            break;
        };
        let bit = match table.name() {
            "romx_projection_meta" => 1,
            "romx_projection_state" => 2,
            "romx_projection_keys" => 4,
            _ => return Err(Error::Unsupported),
        };
        if seen & bit != 0 {
            return Err(Error::Unsupported);
        }
        seen |= bit;
    }
    if seen != 7
        || tables.next().is_some()
        || transaction
            .list_multimap_tables()
            .map_err(|_| Error::Storage)?
            .next()
            .is_some()
    {
        return Err(Error::Unsupported);
    }
    let metadata = transaction
        .open_table(META)
        .map_err(|_| Error::Unsupported)?;
    let mut entries = metadata.iter().map_err(|_| Error::Storage)?;
    let (key, value) = entries
        .next()
        .ok_or(Error::Unsupported)?
        .map_err(|_| Error::Storage)?;
    if key.value() != b"profile" || entries.next().is_some() {
        return Err(Error::Unsupported);
    }
    codec::validate_profile(value.value(), profile)?;
    let control = transaction
        .open_table(STATE)
        .map_err(|_| Error::Unsupported)?;
    let mut count = 0;
    for entry in control.iter().map_err(|_| Error::Storage)? {
        count += 1;
        if count > 3 {
            return Err(Error::Unsupported);
        }
        let (key, value) = entry.map_err(|_| Error::Storage)?;
        if !matches!(key.value(), b"checkpoint" | b"pending" | b"completed_page") {
            return Err(Error::Unsupported);
        }
        if value.value().len() > limits::RECORD {
            return Err(Error::TooLarge);
        }
    }
    let state = snapshot(transaction)?;
    validate_control(&state, profile)?;
    let keys = transaction
        .open_table(KEYS)
        .map_err(|_| Error::Unsupported)?;
    let mut segment = 0;
    for entry in keys.iter().map_err(|_| Error::Storage)? {
        let (key, value) = entry.map_err(|_| Error::Storage)?;
        let key = codec::decode_key(key.value())?;
        if state.checkpoint.cursor(&key.kind).is_none() || state.completed.is_none() {
            return Err(Error::Corrupt);
        }
        codec::decode_key_state(value.value())?;
        segment += 1;
        if segment == limits::SCAN_SEGMENT {
            segment = 0;
            check()?;
            std::thread::yield_now();
        }
    }
    check()?;
    if let Some(page) = &state.pending {
        for operation in &page.operations {
            if let Some(value) = keys
                .get(codec::key(&operation.key).as_slice())
                .map_err(|_| Error::Storage)?
            {
                let prior = codec::decode_key_state(value.value())?;
                if prior.revision > operation.revision
                    || (prior.revision == operation.revision && prior != operation.key_state())
                {
                    return Err(Error::Corrupt);
                }
            }
        }
    }
    if let Some(page) = &state.completed_page {
        for operation in &page.operations {
            let value = keys
                .get(codec::key(&operation.key).as_slice())
                .map_err(|_| Error::Storage)?
                .ok_or(Error::Corrupt)?;
            if codec::decode_key_state(value.value())? != operation.key_state() {
                return Err(Error::Corrupt);
            }
        }
    }
    check()?;
    Ok(())
}

fn validate_control(state: &StoreSnapshot, profile: &ProjectionProfile) -> Result<()> {
    let previous_sequence = if let Some((prepare, complete)) = &state.completed {
        let page = state.completed_page.as_ref().ok_or(Error::Corrupt)?;
        if prepare.transition != 1
            || complete.transition != 2
            || prepare.fingerprint != complete.fingerprint
            || prepare.sequence.checked_add(1) != Some(complete.sequence)
            || page.expected.sequence.checked_add(1) != Some(prepare.sequence)
            || page.expected.sequence.checked_add(2) != Some(complete.sequence)
            || codec::page_digest(profile, page) != complete.fingerprint
        {
            return Err(Error::Corrupt);
        }
        let mut expected = page.expected.clone();
        expected.sequence = complete
            .sequence
            .checked_add(u64::from(state.pending.is_some()))
            .ok_or(Error::Corrupt)?;
        *expected
            .cursors
            .iter_mut()
            .find(|cursor| cursor.kind == page.next.kind)
            .ok_or(Error::Corrupt)? = page.next.clone();
        if expected != state.checkpoint {
            return Err(Error::Corrupt);
        }
        complete.sequence
    } else {
        if state.completed_page.is_some() {
            return Err(Error::Corrupt);
        }
        0
    };
    if let Some(page) = &state.pending {
        let mut prepared = page.expected.clone();
        prepared.sequence = prepared.sequence.checked_add(1).ok_or(Error::Corrupt)?;
        if prepared != state.checkpoint || page.expected.sequence != previous_sequence {
            return Err(Error::Corrupt);
        }
        // Digest construction must remain bounded after schema validation.
        let _ = codec::page_digest(profile, page);
    } else if state.checkpoint.sequence != previous_sequence {
        return Err(Error::Corrupt);
    }
    Ok(())
}

fn copy_contents(
    source: &mut File,
    output: &mut File,
    original_size: u64,
    check: &mut impl FnMut() -> Result<()>,
) -> Result<u64> {
    let mut copied = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        check()?;
        let count = source.read(&mut buffer).map_err(|_| Error::Storage)?;
        if count == 0 {
            break;
        }
        copied = copied.checked_add(count as u64).ok_or(Error::TooLarge)?;
        if copied > limits::RECOVERY_FILE || copied > original_size {
            return Err(Error::TooLarge);
        }
        output
            .write_all(&buffer[..count])
            .map_err(|_| Error::Storage)?;
    }
    Ok(copied)
}

#[cfg(test)]
#[path = "preflight_cancellation.rs"]
mod cancellation_tests;
