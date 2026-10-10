use crate::Error;
/// Validated aggregate archive byte/record budgets. These are not physical operation deadlines.
#[derive(Clone, Copy, Debug, Default)]
pub struct Limits {
    pub(crate) native: rom_backup::BackupLimits,
}
impl Limits {
    /// Require positive budgets, at most128MiB and400000 aggregate records.
    pub fn new(max_bytes: usize, max_records: usize) -> Result<Self, Error> {
        if max_bytes == 0 || max_records == 0 {
            return Err(Error::Invalid);
        }
        if max_bytes > 128 * 1024 * 1024 || max_records > 400000 {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            native: rom_backup::BackupLimits {
                max_bytes,
                max_records,
            },
        })
    }
}
