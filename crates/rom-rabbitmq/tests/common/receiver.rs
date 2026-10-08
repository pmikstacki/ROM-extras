use rom::*;
use std::sync::Arc;
#[derive(Clone, Resource)]
#[resource(name = "received-notices")]
struct Received {
    count: u64,
}
const APPLY: Action<Received, bool> = Action::new("apply", |state, payload| {
    if payload {
        return Err(Error::invalid("received-notices", "payload"));
    }
    state.count += 1;
    Ok(vec![])
});
pub struct Receiver {
    rt: Runtime,
}
impl Receiver {
    pub async fn open(redb: bool, path: &std::path::Path, initialize: bool) -> Self {
        let store: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
        };
        let rt = Runtime::builder()
            .resource(
                Received::definition()
                    .allow_all_fields()
                    .policy(|_, _, _| true)
                    .action(APPLY),
            )
            .build(store, Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        if initialize {
            rt.execute(
                &Self::actor(),
                Command::create("counter", Received { count: 0 }).idempotency("seed"),
            )
            .await
            .unwrap();
        }
        Self { rt }
    }
    fn actor() -> Actor {
        Actor::trusted("fixture", "consumer").with_kind(PrincipalKind::Service)
    }
    pub async fn accept(&self, id: &str, body: &[u8]) {
        assert_eq!(body, b"false");
        self.rt
            .execute(
                &Self::actor(),
                Command::action("counter", APPLY, false)
                    .at_revision(1)
                    .idempotency(id),
            )
            .await
            .unwrap();
    }
    pub async fn count(&self) -> u64 {
        self.rt
            .read::<Received>(&Self::actor(), "counter")
            .await
            .unwrap()
            .value
            .unwrap()
            .count
    }
    pub async fn shutdown(&self) {
        self.rt.shutdown().await.unwrap();
    }
}
