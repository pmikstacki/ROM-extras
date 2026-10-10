use crate::Error;
/// Whole-checkpoint bounds. Native and synchronous cleanup have no hard deadline.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Public archive validation bounds, at most128MiB and400000 aggregate records.
    pub archive: rom_backup::BackupLimits,
    /// At most10000 current Ready Blob bindings.
    pub max_ready: usize,
    /// At most16MiB per object, matching the pinned public BlobService capability.
    pub max_object_bytes: usize,
    /// At most1GiB across all stores, including equal-content publications.
    pub max_total_bytes: u64,
    /// At most4MiB encoded or input private inventory JSON.
    pub max_json_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            archive: rom_backup::BackupLimits::default(),
            max_ready: 10000,
            max_object_bytes: 16 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
            max_json_bytes: 4 * 1024 * 1024,
        }
    }
}
impl Limits {
    pub(crate) fn validate(self) -> Result<(), Error> {
        if self.archive.max_bytes == 0 || self.archive.max_records == 0 || self.max_json_bytes == 0
        {
            return Err(Error::Invalid);
        }
        if self.archive.max_bytes > 128 * 1024 * 1024
            || self.archive.max_records > 400000
            || self.max_ready > 10000
            || self.max_object_bytes > 16 * 1024 * 1024
            || self.max_total_bytes > 1024 * 1024 * 1024
            || self.max_json_bytes > 4 * 1024 * 1024
        {
            return Err(Error::TooLarge);
        }
        Ok(())
    }
    pub(crate) fn recovery(self) -> rom_blob_recovery::Limits {
        rom_blob_recovery::Limits {
            max_entries: self.max_ready,
            max_object_bytes: self.max_object_bytes,
            max_total_bytes: self.max_total_bytes,
            max_json_bytes: self.max_json_bytes,
        }
    }
}
