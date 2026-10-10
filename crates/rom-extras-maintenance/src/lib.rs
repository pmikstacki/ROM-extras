//! Host-only native ROM maintenance. Reports omit private archive data.
//! Reuses public archive validation and native publication; external blobs/deliveries remain excluded.
mod archive;
mod backup;
mod error;
mod limits;
mod native;
mod report;
pub use archive::{inspect, preview_migration};
pub use backup::{BackupSource, backup};
pub use error::Error;
pub use limits::Limits;
pub use native::{migrate_from, restore};
pub use report::{Counts, MigrationPreview, Report};
pub use rom_backup::{Backend, MigrationPlan, ResourceMigration};
