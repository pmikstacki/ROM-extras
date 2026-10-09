use crate::Error;
use rom_import::{ActionPlan, PreparedAction};
use std::{
    fs::File,
    io::{Read, Seek},
};
/// Consume a host-opened regular file from its beginning under the plan's byte limit.
/// The host owns path selection and opening. Filesystem calls have no hard deadline.
/// Concurrent mutation is not prevented; the approved digest binds the actual bytes.
pub fn prepare_file(plan: &ActionPlan, mut file: File) -> Result<PreparedAction, Error> {
    let metadata = file.metadata().map_err(|_| Error::Unavailable)?;
    if !metadata.is_file() {
        return Err(Error::UnsupportedSource);
    }
    let limit = plan.max_document_bytes();
    if metadata.len() > limit as u64 {
        return Err(Error::TooLarge);
    }
    file.rewind().map_err(|_| Error::Unavailable)?;
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() > limit {
        return Err(Error::TooLarge);
    }
    plan.prepare(&bytes).map_err(Error::Admission)
}
