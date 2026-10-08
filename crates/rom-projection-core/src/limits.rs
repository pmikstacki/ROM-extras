//! Fixed first-profile admission checks.
use crate::{Error, Result};
use rom::{JournalCursor, Key};
pub(crate) const IDENTIFIER: usize = 128;
pub(crate) const RESOURCE_ID: usize = 1024;
pub(crate) const OPERATIONS: usize = 64;
pub(crate) const KINDS: usize = 64;
pub(crate) const RECORD: usize = 128 * 1024;
pub(crate) const KEY_RECORD: usize = 2 * 1024;
pub(crate) const SCAN_SEGMENT: usize = 256;
pub(crate) const CACHE: usize = 8 * 1024 * 1024;
pub(crate) const RECOVERY_FILE: u64 = 256 * 1024 * 1024;

pub(crate) fn identifier(value: &str, bound: usize) -> Result<()> {
    if value.len() > bound {
        return Err(Error::TooLarge);
    }
    if value.is_empty() || value.contains('\0') {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub(crate) fn key(key: &Key) -> Result<()> {
    identifier(&key.kind, IDENTIFIER)?;
    identifier(&key.id, RESOURCE_ID)
}
pub(crate) fn cursor(cursor: &JournalCursor) -> Result<()> {
    identifier(&cursor.kind, IDENTIFIER)?;
    identifier(&cursor.generation, IDENTIFIER)
}
