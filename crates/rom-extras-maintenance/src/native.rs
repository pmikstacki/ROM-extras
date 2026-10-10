use crate::{Backend, Error, Limits, MigrationPlan};
use std::{path::Path, sync::Arc};
/// Restore only to a fresh destination using native validation and staged publication.
/// Destination claims/cursors are fenced; the host must independently stop/cut over the old deployment.
/// A disabled profile fails before filesystem access. Unknown never causes an automatic retry.
pub fn restore(
    backend: Backend,
    archive: &Path,
    destination: &Path,
    limits: Limits,
) -> Result<Arc<dyn rom::Storage>, Error> {
    let _ = (archive, destination, limits);
    match backend {
        Backend::Sqlite => {
            #[cfg(feature = "sqlite")]
            {
                rom_sqlite::Sqlite::restore_from(archive, destination, limits.native)
                    .map(|s| Arc::new(s) as Arc<dyn rom::Storage>)
                    .map_err(Error::from)
            }
            #[cfg(not(feature = "sqlite"))]
            {
                Err(Error::Unsupported)
            }
        }
        Backend::Redb => {
            #[cfg(feature = "redb")]
            {
                rom_redb::Redb::restore_from(archive, destination, limits.native)
                    .map(|s| Arc::new(s) as Arc<dyn rom::Storage>)
                    .map_err(Error::from)
            }
            #[cfg(not(feature = "redb"))]
            {
                Err(Error::Unsupported)
            }
        }
    }
}
/// Apply a compiled host schema plan to an offline existing source and publish a fresh destination.
/// Upstream native ownership protects both paths. Converters must be pure/bounded; no callback replay.
/// The source deployment must be closed; successful destination publication does not disable it.
pub fn migrate_from(
    backend: Backend,
    source: &Path,
    destination: &Path,
    plan: &MigrationPlan,
    limits: Limits,
) -> Result<Arc<dyn rom::Storage>, Error> {
    let _ = (source, destination, plan, limits);
    match backend {
        Backend::Sqlite => {
            #[cfg(feature = "sqlite")]
            {
                rom_sqlite::Sqlite::migrate_from(source, destination, plan, limits.native)
                    .map(|s| Arc::new(s) as Arc<dyn rom::Storage>)
                    .map_err(Error::from)
            }
            #[cfg(not(feature = "sqlite"))]
            {
                Err(Error::Unsupported)
            }
        }
        Backend::Redb => {
            #[cfg(feature = "redb")]
            {
                rom_redb::Redb::migrate_from(source, destination, plan, limits.native)
                    .map(|s| Arc::new(s) as Arc<dyn rom::Storage>)
                    .map_err(Error::from)
            }
            #[cfg(not(feature = "redb"))]
            {
                Err(Error::Unsupported)
            }
        }
    }
}
