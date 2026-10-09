//! Actual durable Runtime behavior; telemetry only receives typed summaries.
use rom::*;
use rom_opentelemetry::{Operation, Outcome, RuntimeMetrics};
use std::{path::Path, sync::Arc, time::Instant};

pub const CANARY: &str = "private-telemetry-fixture-resource-actor-error";
#[derive(Clone, Resource)]
#[resource(name = "telemetry-counter")]
struct Counter {
    count: u64,
    private_value: String,
}
const CHANGE: Action<Counter, bool> = Action::new("change", |state, accepted| {
    if !accepted {
        return Err(Error::invalid(CANARY, CANARY));
    }
    state.count += 1;
    Ok(vec![])
});
pub trait Observer {
    type Scope<'a>
    where
        Self: 'a;
    fn start(&self) -> Self::Scope<'_>;
    fn finish<'a>(scope: Self::Scope<'a>, outcome: Outcome)
    where
        Self: 'a;
}
struct MetricObserver<'a>(&'a RuntimeMetrics);
impl Observer for MetricObserver<'_> {
    type Scope<'a>
        = (&'a RuntimeMetrics, Instant)
    where
        Self: 'a;
    fn start(&self) -> Self::Scope<'_> {
        (self.0, Instant::now())
    }
    fn finish<'a>((metrics, start): Self::Scope<'a>, outcome: Outcome)
    where
        Self: 'a,
    {
        metrics.record_operation(Operation::Action, outcome, start.elapsed());
    }
}
async fn observed<O: Observer>(
    runtime: &Runtime,
    actor: &Actor,
    command: Command<Counter>,
    observer: &O,
) -> Result<Snapshot<Counter>> {
    let scope = observer.start();
    let result = runtime.execute(actor, command).await;
    O::finish(scope, Outcome::from_result(&result));
    result
}
pub async fn qualify(path: &Path, redb: bool, metrics: &RuntimeMetrics) {
    qualify_with(path, redb, metrics, &MetricObserver(metrics)).await;
}
pub async fn qualify_with<O: Observer>(
    path: &Path,
    redb: bool,
    metrics: &RuntimeMetrics,
    observer: &O,
) {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    };
    let runtime = Runtime::builder()
        .resource(
            Counter::definition()
                .allow_all_fields()
                .policy(|actor, _, _| actor.principal_kind() != PrincipalKind::Service)
                .action(CHANGE),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let owner = Actor::trusted("fixture", CANARY);
    let denied = owner.clone().with_kind(PrincipalKind::Service);
    metrics.record_status(&runtime.status()).unwrap();
    let seed = observed(
        &runtime,
        &owner,
        Command::create(
            CANARY,
            Counter {
                count: 0,
                private_value: CANARY.into(),
            },
        )
        .idempotency("seed"),
        observer,
    )
    .await
    .unwrap();
    assert_eq!(seed.revision, 1);
    let rejected = observed(
        &runtime,
        &owner,
        Command::action(CANARY, CHANGE, false)
            .at_revision(1)
            .idempotency("rejected"),
        observer,
    )
    .await;
    assert!(matches!(rejected, Err(Error::Invalid { .. })));
    let changed = observed(
        &runtime,
        &owner,
        Command::action(CANARY, CHANGE, true)
            .at_revision(1)
            .idempotency("change"),
        observer,
    )
    .await
    .unwrap();
    assert_eq!(changed.revision, 2);
    assert_eq!(changed.value.as_ref().unwrap().count, 1);
    let replay = observed(
        &runtime,
        &owner,
        Command::action(CANARY, CHANGE, true)
            .at_revision(1)
            .idempotency("change"),
        observer,
    )
    .await
    .unwrap();
    assert_eq!(replay.id, CANARY);
    assert_eq!(replay.revision, 2);
    assert_eq!(replay.value.unwrap().count, 1);
    let refused = observed(
        &runtime,
        &denied,
        Command::action(CANARY, CHANGE, true)
            .at_revision(2)
            .idempotency("denied"),
        observer,
    )
    .await;
    assert!(matches!(refused, Err(Error::Denied)));
    runtime.shutdown().await.unwrap();
    assert_eq!(runtime.status().unwrap().intake, IntakeState::Stopped);
    metrics.record_status(&runtime.status()).unwrap();
}
