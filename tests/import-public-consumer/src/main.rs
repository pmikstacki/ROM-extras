//! Public, independent consumer with actual SQLite and redb persistence.
use rom::*;
use rom_import::{ActionBinding, ActionPlan, Limits, SourceGrant};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::Arc};
const ID: &str = "settings.exact[0]";
#[derive(Clone, Resource)]
#[resource(name = "import-controls")]
struct Control {
    generation: u64,
}
#[derive(Clone, Resource)]
#[resource(name = "imported-values")]
struct Item {
    computed: u64,
    retained: bool,
}
const INGEST: Action<Item, u64> = Action::new("ingest", |value, input| {
    value.computed = input.checked_mul(2).ok_or(Error::TooLarge)?;
    Ok(vec![])
});
fn actor() -> Actor {
    Actor::trusted("host", "worker").with_kind(PrincipalKind::Service)
}
fn origins() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("computed".into(), "action-computed".into()),
        ("retained".into(), "initial".into()),
    ])
}
fn plan(
    bytes: &[u8],
    expected: u64,
    origins: BTreeMap<String, String>,
    idempotency: &str,
) -> ActionPlan {
    ActionPlan::trusted(
        ActionBinding {
            target: Key {
                kind: Item::KIND.into(),
                id: ID.into(),
            },
            action: "ingest".into(),
            expected: Some(expected),
            idempotency: idempotency.into(),
            retry_epoch: 0,
        },
        SourceGrant {
            source: "deployment".into(),
            generation: 2,
            output_origins: origins,
            condition: RevisionCondition {
                key: Key {
                    kind: Control::KIND.into(),
                    id: "source".into(),
                },
                revision: 1,
            },
            valid_until: u64::MAX,
            expected_sha256: Sha256::digest(bytes).into(),
        },
        Limits::default(),
    )
    .unwrap()
}
fn open(path: &Path, redb: bool) -> Runtime {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    };
    Runtime::builder()
        .resource(
            Control::definition()
                .policy(|a, _, _| a.subject == "worker")
                .allow_all_fields(),
        )
        .resource(
            Item::definition()
                .policy(|a, _, _| a.subject == "worker")
                .allow_all_fields()
                .source_owner("deployment")
                .source_metadata_policy(|a| a.principal_kind() == PrincipalKind::Service)
                .action(INGEST),
        )
        .build(storage, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}
async fn apply(
    runtime: &Runtime,
    actor: &Actor,
    prepared: &rom_import::PreparedAction,
) -> Result<ProjectedView> {
    let (invocation, permit) = prepared.request();
    runtime.invoke_sourced(actor, invocation, permit).await
}
async fn qualify(path: &Path, redb: bool) {
    let runtime = open(path, redb);
    let worker = actor();
    runtime
        .execute(
            &worker,
            Command::create("source", Control { generation: 2 }).idempotency("control"),
        )
        .await
        .unwrap();
    let seed = SourcePermit::trusted(
        Key {
            kind: Item::KIND.into(),
            id: ID.into(),
        },
        SourceProvenance {
            source: "deployment".into(),
            version: "initial".into(),
            generation: 1,
            field_origins: origins(),
        },
        RevisionCondition {
            key: Key {
                kind: Control::KIND.into(),
                id: "source".into(),
            },
            revision: 1,
        },
        u64::MAX,
    )
    .unwrap();
    runtime
        .invoke_sourced(
            &worker,
            Command::create(
                ID,
                Item {
                    computed: 0,
                    retained: true,
                },
            )
            .idempotency("seed")
            .into(),
            seed,
        )
        .await
        .unwrap();
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
