//! Shared authored Resource fixture using only published ROM contracts.
use rom::*;
use rom_import::{ActionBinding, ActionPlan, Limits, SourceGrant};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::Arc};
pub(crate) const ID: &str = "settings.exact[0]";
#[derive(Clone, Resource)]
#[resource(name = "import-controls")]
pub(crate) struct Control {
    pub(crate) generation: u64,
}
#[derive(Clone, Resource)]
#[resource(name = "imported-values")]
pub(crate) struct Item {
    pub(crate) computed: u64,
    pub(crate) retained: bool,
}
pub(crate) const INGEST: Action<Item, u64> = Action::new("ingest", |value, input| {
    value.computed = input.checked_mul(2).ok_or(Error::TooLarge)?;
    Ok(vec![])
});
pub(crate) fn actor() -> Actor {
    Actor::trusted("host", "worker").with_kind(PrincipalKind::Service)
}
pub(crate) fn origins() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("computed".into(), "action-computed".into()),
        ("retained".into(), "initial".into()),
    ])
}
pub(crate) fn plan(
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
pub(crate) fn open(path: &Path, redb: bool) -> Runtime {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    };
    runtime_for(storage)
}
pub(crate) fn runtime_for(storage: Arc<dyn Storage>) -> Runtime {
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
pub(crate) async fn apply(
    runtime: &Runtime,
    actor: &Actor,
    prepared: &rom_import::PreparedAction,
) -> Result<ProjectedView> {
    let (invocation, permit) = prepared.request();
    runtime.invoke_sourced(actor, invocation, permit).await
}

pub(crate) async fn seed(runtime: &Runtime) {
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
}
