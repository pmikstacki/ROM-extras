use crate::{Counts, Error, Inventory, Limits};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
pub(crate) fn read_private(path: &Path, max: usize) -> Result<Vec<u8>, Error> {
    if !cfg!(unix) {
        return Err(Error::Unsupported);
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|_| Error::Unavailable)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(Error::Invalid);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::Invalid);
        }
    }
    if metadata.len() > max as u64 {
        return Err(Error::TooLarge);
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| Error::Unavailable)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() > max {
        return Err(Error::TooLarge);
    }
    Ok(bytes)
}
impl Inventory {
    /// Publish sensitive version-one JSON through public private staging, synchronization and no-overwrite linking.
    /// Synchronous I/O has no hard deadline. Unknown preserves the original destination without automatic retry.
    pub fn publish(&self, path: &Path) -> Result<Counts, Error> {
        let bytes = self.encode()?;
        let stage = rom_backup::Stage::new(path).map_err(Error::from)?;
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(stage.path())
            .map_err(|_| Error::Unavailable)?;
        file.write_all(&bytes).map_err(|_| Error::Unavailable)?;
        drop(file);
        stage.publish().map_err(Error::from)?;
        Ok(self.counts)
    }
    /// Read bounded private regular-file JSON under a trusted parent. Validate provenance and expected checkpoint separately.
    pub fn read(path: &Path, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        Self::decode(&read_private(path, limits.max_json_bytes)?, limits)
    }
}
