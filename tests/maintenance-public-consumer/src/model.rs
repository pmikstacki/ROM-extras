use rom::*;
use rom_extras_maintenance::{MigrationPlan, ResourceMigration};
use std::sync::Arc;
pub const ID: &str = "private.resource/id[0]";
#[derive(Clone, Resource)]
#[resource(name = "maintenance-items")]
pub struct Before {
    pub amount: String,
    pub enabled: bool,
}
#[derive(Clone, Resource)]
#[resource(name = "maintenance-items", version = 2)]
pub struct After {
    pub amount: u64,
    pub enabled: bool,
    pub note: Option<String>,
}
pub fn plan() -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|v| {
            Ok(After {
                amount: v
                    .amount
                    .parse()
                    .map_err(|_| Error::invalid("private-converter-canary", "secret-field"))?,
                enabled: v.enabled,
                note: None,
            })
        })
        .unwrap(),
    ])
    .unwrap()
}
pub fn failing_plan() -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|_| {
            Err(Error::invalid("private-converter-canary", "secret-field"))
        })
        .unwrap(),
    ])
    .unwrap()
}
pub fn actor() -> Actor {
    Actor::trusted("host", "maintenance-worker")
}
pub fn runtime(storage: Arc<dyn Storage>) -> Runtime {
    Runtime::builder()
        .resource(
            Before::definition()
                .policy(|a, _, _| a.subject == "maintenance-worker")
                .allow_all_fields()
                .action(Action::new("increment", |v, input: u64| {
                    v.amount = input.to_string();
                    Ok(vec![])
                })),
        )
        .build(storage, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}
pub fn after_runtime(storage: Arc<dyn Storage>) -> Runtime {
    Runtime::builder()
        .resource(
            After::definition()
                .policy(|a, _, _| a.subject == "maintenance-worker")
                .allow_all_fields(),
        )
        .build(storage, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}
pub async fn seed(runtime: &Runtime) {
    runtime
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
    runtime
        .execute(
            &actor(),
            Command::action(
                ID,
                Action::<Before, u64>::new("increment", |v, input| {
                    v.amount = input.to_string();
                    Ok(vec![])
                }),
                42,
            )
            .at_revision(1)
            .idempotency("private-action-original"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &actor(),
            Command::create(
                "tombstone.exact",
                Before {
                    amount: "0".into(),
                    enabled: false,
                },
            )
            .idempotency("private-delete-create"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &actor(),
            Command::<Before>::delete("tombstone.exact")
                .at_revision(1)
                .idempotency("private-delete-original"),
        )
        .await
        .unwrap();
}
