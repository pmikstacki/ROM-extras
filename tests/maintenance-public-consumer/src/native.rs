use crate::model::*;
use rom::{Command, Error as RomError, Resource, Storage};
use rom_extras_maintenance::{
    Backend, BackupSource, Error, Limits, backup, inspect, migrate_from, preview_migration, restore,
};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc};
fn digest(path: &Path) -> Vec<u8> {
    Sha256::digest(std::fs::read(path).unwrap()).to_vec()
}
pub async fn run(root: &Path) {
    qualify(
        root,
        Backend::Sqlite,
        Arc::new(rom_sqlite::Sqlite::open(root.join("sqlite-source.db")).unwrap()),
    )
    .await;
    qualify(
        root,
        Backend::Redb,
        Arc::new(rom_redb::Redb::open(root.join("redb-source.db")).unwrap()),
    )
    .await;
}
async fn qualify<S: Storage + BackupSource>(root: &Path, backend: Backend, source: Arc<S>) {
    let label = if backend == Backend::Sqlite {
        "sqlite"
    } else {
        "redb"
    };
    let archive = root.join(format!("{label}-archive.rombk"));
    let source_path = root.join(format!("{label}-source.db"));
    let destination = root.join(format!("{label}-restored.db"));
    let migrated_path = root.join(format!("{label}-migrated.db"));
    let runtime = runtime(source.clone());
    seed(&runtime).await;
    let old_cursor = source.journal_head(Before::KIND).unwrap();
    let identity = source
        .journal(Before::KIND, None, 10, 100000)
        .unwrap()
        .events[1]
        .identity
        .clone();
    let original = source.receipt(&identity).unwrap().unwrap();
    let facts = backup(source.as_ref(), &archive, Limits::default()).unwrap();
    assert_eq!(facts.counts.rows, 2);
    assert_eq!(facts.counts.receipts, 4);
    assert_eq!(facts.counts.events, 4);
    assert!(!facts.external_blobs_included && !facts.external_deliveries_included);
    assert_eq!(
        inspect(backend, &archive, Limits::default()).unwrap(),
        facts
    );
    assert!(!format!("{facts:?}").contains("private"));
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&archive).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        backup(source.as_ref(), &archive, Limits::default()),
        Err(Error::Conflict)
    );
    let before_archive = digest(&archive);
    let preview = preview_migration(backend, &archive, &plan(), Limits::default()).unwrap();
    assert_eq!(preview.changed_schemas, 1);
    assert_eq!(preview.historical_replay_codecs, 4);
    assert_eq!(before_archive, digest(&archive));
    assert_eq!(
        preview_migration(backend, &archive, &failing_plan(), Limits::default()),
        Err(Error::Invalid)
    );
    assert!(!format!("{:?}", Error::Invalid).contains("canary"));
    assert!(matches!(
        migrate_from(
            backend,
            &source_path,
            &migrated_path,
            &plan(),
            Limits::default()
        ),
        Err(Error::Conflict)
    ));
    assert!(!migrated_path.exists());
    let restored = restore(backend, &archive, &destination, Limits::default()).unwrap();
    assert_eq!(
        restored
            .load(&rom::Key {
                kind: Before::KIND.into(),
                id: ID.into()
            })
            .unwrap(),
        source
            .load(&rom::Key {
                kind: Before::KIND.into(),
                id: ID.into()
            })
            .unwrap()
    );
    assert_eq!(restored.receipt(&identity).unwrap().unwrap(), original);
    assert!(matches!(
        restored.journal(Before::KIND, Some(&old_cursor), 10, 100000),
        Err(RomError::HistoryGap)
    ));
    assert!(matches!(
        restore(backend, &archive, &destination, Limits::default()),
        Err(Error::Conflict)
    ));
    let restored_runtime = runtime_for_before(restored.clone());
    assert_eq!(
        restored_runtime
            .read::<Before>(&actor(), ID)
            .await
            .unwrap()
            .revision,
        2
    );
    assert!(
        restored_runtime
            .read::<Before>(&actor(), "tombstone.exact")
            .await
            .is_err()
    );
    let replay = restored_runtime
        .execute(
            &actor(),
            Command::create(
                ID,
                Before {
                    amount: "41".into(),
                    enabled: false,
                },
            )
            .idempotency("private-create-original"),
        )
        .await
        .unwrap();
    assert_eq!(replay.revision, 1);
    restored_runtime.shutdown().await.unwrap();
    drop(restored_runtime);
    drop(restored);
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(source);
    let source_digest = digest(&source_path);
    assert!(matches!(
        migrate_from(
            backend,
            &source_path,
            &root.join(format!("{label}-failed-migration.db")),
            &failing_plan(),
            Limits::default()
        ),
        Err(Error::Invalid)
    ));
    assert_eq!(source_digest, digest(&source_path));
    let migrated = migrate_from(
        backend,
        &source_path,
        &migrated_path,
        &plan(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(source_digest, digest(&source_path));
    let receipt = migrated.receipt(&identity).unwrap().unwrap();
    assert_eq!(receipt.identity, original.identity);
    assert_eq!(receipt.fingerprint, original.fingerprint);
    assert_eq!(receipt.row.revision, original.row.revision);
    assert_eq!(receipt.replay_version, Some(1));
    assert_eq!(
        receipt.row.value,
        Some(rom::json!({"amount":42,"enabled":false,"note":null}))
    );
    let migrated_runtime = after_runtime(migrated.clone());
    let current = migrated_runtime.read::<After>(&actor(), ID).await.unwrap();
    assert_eq!(current.revision, 2);
    assert_eq!(current.value.as_ref().unwrap().amount, 42);
    assert!(!current.value.as_ref().unwrap().enabled);
    assert_eq!(current.value.as_ref().unwrap().note, None);
    migrated_runtime.shutdown().await.unwrap();
    drop(migrated_runtime);
    drop(migrated);
    let reopen: Arc<dyn Storage> = if backend == Backend::Sqlite {
        Arc::new(rom_sqlite::Sqlite::open(&migrated_path).unwrap())
    } else {
        Arc::new(rom_redb::Redb::open(&migrated_path).unwrap())
    };
    assert_eq!(reopen.receipt(&identity).unwrap().unwrap(), receipt);
    drop(reopen);
    negatives(root, backend, &archive);
    println!(
        "{label}: actual create/action/tombstone backup, full inspection, private archive, restore/replay/cursor fence, typed preview/migration and reopen passed"
    );
}
fn runtime_for_before(s: Arc<dyn Storage>) -> rom::Runtime {
    crate::model::runtime(s)
}
fn negatives(root: &Path, backend: Backend, archive: &Path) {
    let other = if backend == Backend::Sqlite {
        Backend::Redb
    } else {
        Backend::Sqlite
    };
    assert_eq!(
        inspect(other, archive, Limits::default()),
        Err(Error::Unsupported)
    );
    assert_eq!(
        inspect(backend, archive, Limits::new(1, 400000).unwrap()),
        Err(Error::TooLarge)
    );
    assert_eq!(
        inspect(backend, archive, Limits::new(128 * 1024 * 1024, 1).unwrap()),
        Err(Error::TooLarge)
    );
    let corrupt = root.join(format!("{backend:?}-corrupt.rombk"));
    let mut bytes = std::fs::read(archive).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    std::fs::write(&corrupt, bytes).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&corrupt, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        inspect(backend, &corrupt, Limits::default()),
        Err(Error::Unavailable)
    );
    let future = root.join(format!("{backend:?}-future.rombk"));
    let mut future_bytes = std::fs::read(archive).unwrap();
    let marker = b"\"archive_version\":6";
    let index = future_bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .unwrap();
    future_bytes[index + marker.len() - 1] = b'7';
    let checksum = Sha256::digest(&future_bytes[52..]);
    future_bytes[20..52].copy_from_slice(&checksum);
    std::fs::write(&future, future_bytes).unwrap();
    std::fs::set_permissions(&future, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        inspect(backend, &future, Limits::default()),
        Err(Error::Unsupported)
    );
    let future_dest = root.join(format!("{backend:?}-future-destination.db"));
    assert!(matches!(
        restore(backend, &future, &future_dest, Limits::default()),
        Err(Error::Unsupported)
    ));
    assert!(!future_dest.exists());
    let public = root.join(format!("{backend:?}-public-permissions.rombk"));
    std::fs::copy(archive, &public).unwrap();
    std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        inspect(backend, &public, Limits::default()),
        Err(Error::Unavailable)
    );
    let link = root.join(format!("{backend:?}-symlink.rombk"));
    std::os::unix::fs::symlink(archive, &link).unwrap();
    assert_eq!(
        inspect(backend, &link, Limits::default()),
        Err(Error::Unavailable)
    );
    let missing = root.join(format!("{backend:?}-missing-source.db"));
    let dest = root.join(format!("{backend:?}-missing-destination.db"));
    assert!(migrate_from(backend, &missing, &dest, &plan(), Limits::default()).is_err());
    assert!(!missing.exists() && !dest.exists());
    assert!(
        !root
            .join(format!("{backend:?}-missing-source.db.rom-owner"))
            .exists()
    );
    assert!(restore(backend, &corrupt, &dest, Limits::default()).is_err());
    assert!(!dest.exists());
}
