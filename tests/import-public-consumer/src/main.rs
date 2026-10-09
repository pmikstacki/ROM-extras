//! Public, independent consumer with actual SQLite and redb persistence.
mod model;
use model::*;
use rom::*;
use std::{collections::BTreeMap, path::Path};
async fn qualify(path: &Path, redb: bool) {
    let runtime = open(path, redb);
    let worker = actor();
    seed(&runtime).await;
    let prepared = plan(b"21", 1, origins(), "import-original")
        .prepare(b"21")
        .unwrap();
    assert!(matches!(
        apply(&runtime, &Actor::trusted("host", "worker"), &prepared).await,
        Err(Error::Denied)
    ));
    let denied = apply(
        &runtime,
        &Actor::trusted("host", "other").with_kind(PrincipalKind::Service),
        &prepared,
    )
    .await;
    assert!(matches!(denied, Err(Error::Denied)), "actual {denied:?}");
    let incomplete = plan(
        b"21",
        1,
        BTreeMap::from([("computed".into(), "action-computed".into())]),
        "import-original",
    )
    .prepare(b"21")
    .unwrap();
    assert!(matches!(
        apply(&runtime, &worker, &incomplete).await,
        Err(Error::Invalid { .. })
    ));
    assert_eq!(runtime.read::<Item>(&worker, ID).await.unwrap().revision, 1);
    let accepted = apply(&runtime, &worker, &prepared).await.unwrap();
    assert_eq!(accepted.revision, 2);
    assert_eq!(
        apply(&runtime, &worker, &prepared).await.unwrap().revision,
        2
    );
    let stale_target = plan(b"21", 3, origins(), "target-stale")
        .prepare(b"21")
        .unwrap();
    assert!(matches!(
        apply(&runtime, &worker, &stale_target).await,
        Err(Error::Conflict)
    ));
    assert_eq!(runtime.read::<Item>(&worker, ID).await.unwrap().revision, 2);
    let different = plan(b"22", 1, origins(), "import-original")
        .prepare(b"22")
        .unwrap();
    assert!(matches!(
        apply(&runtime, &worker, &different).await,
        Err(Error::IdentityMismatch)
    ));
    let meta = runtime
        .source_provenance(&worker, Item::KIND, ID)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        meta.version,
        "sha256:6f4b6612125fb3a0daecd2799dfd6c9c299424fd920f9b308110a2c1fbd8f443"
    );
    assert_eq!(meta.field_origins, origins());
    assert_eq!(meta.generation, 2);
    assert!(matches!(
        runtime
            .source_provenance(&Actor::trusted("host", "worker"), Item::KIND, ID)
            .await,
        Err(Error::Denied)
    ));
    let row = runtime
        .read::<Item>(&worker, ID)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(row.computed, 42);
    assert!(row.retained);
    runtime.shutdown().await.unwrap();
    drop(runtime);
    let runtime = open(path, redb);
    assert_eq!(
        apply(&runtime, &worker, &prepared).await.unwrap().revision,
        2
    );
    assert_eq!(
        runtime
            .source_provenance(&worker, Item::KIND, ID)
            .await
            .unwrap(),
        Some(meta)
    );
    runtime
        .execute(
            &worker,
            Command::replace("source", Control { generation: 3 })
                .at_revision(1)
                .idempotency("new-generation"),
        )
        .await
        .unwrap();
    assert!(matches!(
        apply(&runtime, &worker, &prepared).await,
        Err(Error::Conflict)
    ));
    // A replay conflict after commit does not undo that committed value.
    assert_eq!(
        runtime
            .read::<Item>(&worker, ID)
            .await
            .unwrap()
            .value
            .unwrap()
            .computed,
        42
    );
    runtime.shutdown().await.unwrap();
}
fn main() {
    let root = std::path::PathBuf::from(std::env::var("ROM_EXTRAS_IMPORT_PATH").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    for redb in [false, true] {
        let backend = if redb { "redb" } else { "sqlite" };
        rt.block_on(qualify(&root.join(backend), redb));
        println!(
            "{backend}: sourced domain action, exact identity, durable attribution, denied authority, output coverage, replay and reopen passed"
        );
    }
}
