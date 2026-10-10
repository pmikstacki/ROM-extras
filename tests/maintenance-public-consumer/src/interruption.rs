//! Upstream test-support native interruption witness, not a production facade fault callback.
use crate::model::plan;
use rom::Error;
use sha2::{Digest, Sha256};
use std::path::Path;
pub fn run(root: &Path) {
    for backend in ["sqlite", "redb"] {
        let source = root.join(format!("{backend}-source.db"));
        let destination = root.join(format!("{backend}-interrupted.db"));
        let before = Sha256::digest(std::fs::read(&source).unwrap());
        let result: rom::Result<()> = if backend == "sqlite" {
            rom_sqlite::Sqlite::migrate_from_observed(
                &source,
                &destination,
                &plan(),
                rom_backup::BackupLimits::default(),
                || Err(Error::Closed),
            )
            .map(drop)
        } else {
            rom_redb::Redb::migrate_from_observed(
                &source,
                &destination,
                &plan(),
                rom_backup::BackupLimits::default(),
                || Err(Error::Closed),
            )
            .map(drop)
        };
        assert!(matches!(result, Err(Error::Closed)));
        assert!(!destination.exists());
        assert_eq!(before, Sha256::digest(std::fs::read(&source).unwrap()));
    }
    println!(
        "Upstream native test-support: interrupted staged migration leaves source bytes unchanged and no published destination on both backends; no production facade fault hook"
    );
}
