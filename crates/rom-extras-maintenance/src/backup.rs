use crate::{Error, Limits, Report};
use std::path::Path;
mod sealed {
    pub trait Source {
        fn export(
            &self,
            archive: &std::path::Path,
            limits: rom_backup::BackupLimits,
        ) -> rom::Result<rom_backup::Manifest>;
    }
    #[cfg(feature = "sqlite")]
    impl Source for rom_sqlite::Sqlite {
        fn export(
            &self,
            path: &std::path::Path,
            limits: rom_backup::BackupLimits,
        ) -> rom::Result<rom_backup::Manifest> {
            self.backup_to(path, limits)
        }
    }
    #[cfg(feature = "redb")]
    impl Source for rom_redb::Redb {
        fn export(
            &self,
            path: &std::path::Path,
            limits: rom_backup::BackupLimits,
        ) -> rom::Result<rom_backup::Manifest> {
            self.backup_to(path, limits)
        }
    }
}
/// An already-open native adapter; no path opener or arbitrary SQL label is admitted.
/// Implementations are sealed to the enabled native profiles.
pub trait BackupSource: sealed::Source {}
#[cfg(feature = "sqlite")]
impl BackupSource for rom_sqlite::Sqlite {}
#[cfg(feature = "redb")]
impl BackupSource for rom_redb::Redb {}
/// Collect a coherent native snapshot and publish a private archive without overwrite.
/// Host authorization is required. Do not automatically retry an Unknown publication result.
pub fn backup(source: &impl BackupSource, archive: &Path, limits: Limits) -> Result<Report, Error> {
    source
        .export(archive, limits.native)
        .map(Report::from)
        .map_err(Error::from)
}
