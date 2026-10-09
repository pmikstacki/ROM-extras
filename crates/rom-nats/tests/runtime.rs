//! Native ROM intent/retry persistence with a real post-commit JetStream acknowledgement loss.
mod common;
use async_nats::jetstream::{
    self,
    stream::{Config, StorageType},
};
use common::AckProxy;
use rom::*;
use rom_delivery_core::PayloadLimit;
use rom_nats::JetStreamDelivery;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
#[derive(Clone, Resource)]
#[resource(name = "nats-notices")]
struct Notice {
    count: u64,
}
const CHANNEL: Channel<bool> = Channel::new("extras-nats", 1);
const NOTIFY: Action<Notice, ()> = Action::new("notify", |state, ()| {
    state.count += 1;
    Ok(vec![CHANNEL.intent(false)])
});
struct TestClock(AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn service() -> Actor {
    Actor::trusted("fixture", "nats").with_kind(PrincipalKind::Service)
}
fn owner() -> Actor {
    Actor::trusted("fixture", "owner")
}
fn open(redb: bool, path: &std::path::Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
fn runtime(
    store: Arc<dyn Storage>,
    adapter: Arc<JetStreamDelivery>,
    clock: Arc<TestClock>,
) -> Runtime {
    Runtime::builder()
        .clock(clock)
        .reaction_limits(ReactionLimits {
            max_attempts: 2,
            ..Default::default()
        })
        .resource(
            Notice::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .field_policy(|actor, access, _, state| {
                    actor.principal_kind() != PrincipalKind::Service
                        || !matches!(access, Access::Read)
                        || state.count != 2
                })
                .action(NOTIFY),
        )
        .channel(CHANNEL, service(), move |delivery| {
            let adapter = adapter.clone();
            async move { adapter.deliver(delivery).await }
        })
        .build(store, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
async fn enqueue(rt: &Runtime, id: &str) {
    rt.execute(
        &owner(),
        Command::create(id, Notice { count: 0 }).idempotency(&format!("seed-{id}")),
    )
    .await
    .unwrap();
    rt.execute(
        &owner(),
        Command::action(id, NOTIFY, ())
            .at_revision(1)
            .idempotency(&format!("notify-{id}")),
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn reopened_runtime_retries_committed_intent_and_rechecks_authority() {
    let url = std::env::var("ROM_EXTRAS_NATS_URL").expect("required fixture URL");
    let fixture = common::fixture_name::selected(false);
    common::fixture_name::assert_endpoint(&fixture, false, &url);
    let token = std::env::var("ROM_EXTRAS_NATS_TOKEN").expect("required fixture token");
    for redb in [false, true] {
        let direct = async_nats::ConnectOptions::with_token(token.clone())
            .connection_timeout(Duration::from_secs(2))
            .connect(url.as_str())
            .await
            .unwrap_or_else(|_| panic!("fixture connect failed"));
        let context = jetstream::new(direct);
        let name = format!(
            "RT_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let subject = format!("extras.{name}");
        let mut stream = context
            .create_stream(Config {
                name: name.clone(),
                subjects: vec![subject.clone()],
                storage: StorageType::File,
                duplicate_window: Duration::from_secs(120),
                max_bytes: 1_048_576,
                ..Default::default()
            })
            .await
            .unwrap();
        let proxy = AckProxy::start(&name);
        let client = async_nats::ConnectOptions::with_token(token.clone())
            .ignore_discovered_servers()
            .connection_timeout(Duration::from_secs(2))
            .connect(&proxy.url)
            .await
            .unwrap_or_else(|_| panic!("proxy connect failed"));
        let adapter = Arc::new(
            JetStreamDelivery::bind(
                jetstream::new(client),
                &name,
                &subject,
                1,
                Duration::from_secs(2),
                PayloadLimit::default(),
            )
            .await
            .unwrap(),
        );
        let path = std::env::temp_dir().join(format!("rom-nats-runtime-{name}"));
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let store = open(redb, &path);
        let rt = runtime(store.clone(), adapter.clone(), clock.clone());
        enqueue(&rt, "retry").await;
        let record = store.reaction_records().unwrap().remove(0);
        let id = record.pending.id;
        assert_eq!(record.state, WorkState::Pending);
        assert_eq!(record.delivery, None);
        assert_eq!(stream.info().await.unwrap().state.messages, 0);
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        let rt = runtime(store.clone(), adapter.clone(), clock.clone());
        let completed = std::sync::atomic::AtomicBool::new(false);
        let processing = async {
            let result = rt.process_work(1).await;
            completed.store(true, Ordering::SeqCst);
            result
        };
        let witness = async {
            tokio::time::timeout(Duration::from_secs(1), async {
                while !proxy.dropped(&name) {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("post-commit PubAck drop must precede Runtime deadline");
            assert_eq!(stream.info().await.unwrap().state.messages, 1);
            let raw = stream.get_raw_message(1).await.unwrap();
            assert_eq!(raw.payload.as_ref(), b"false");
            assert_eq!(raw.headers.get("Nats-Msg-Id").unwrap().as_str(), id);
            assert!(
                !completed.load(Ordering::SeqCst),
                "broker witness must complete while Runtime delivery remains pending"
            );
        };
        let (processed, ()) = tokio::join!(processing, witness);
        assert_eq!(processed.unwrap(), 1);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.pending.id, id);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Unknown));
        assert_eq!(record.state, WorkState::Pending);
        assert_eq!(record.attempts, 1);
        assert_eq!(rt.process_work(1).await.unwrap(), 0);
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        assert_eq!(
            store.reaction_records().unwrap()[0].delivery,
            Some(DeliveryOutcome::Unknown)
        );
        clock.0.store(1, Ordering::SeqCst);
        let rt = runtime(store.clone(), adapter.clone(), clock.clone());
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.pending.id, id);
        assert_eq!(record.attempts, 2);
        assert_eq!(record.state, WorkState::Done);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Accepted));
        assert_eq!(stream.info().await.unwrap().state.messages, 1);
        enqueue(&rt, "source-denied").await;
        rt.execute(
            &owner(),
            Command::replace("source-denied", Notice { count: 2 })
                .at_revision(2)
                .idempotency("revoke-source"),
        )
        .await
        .unwrap();
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        enqueue(&rt, "service-denied").await;
        rt.revoke(&service());
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        assert_eq!(stream.info().await.unwrap().state.messages, 1);
        let records = store.reaction_records().unwrap();
        assert_eq!(
            records
                .iter()
                .filter(
                    |r| r.state == WorkState::Stopped(StopReason::Denied) && r.delivery.is_none()
                )
                .count(),
            2
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        let records = store.reaction_records().unwrap();
        assert!(records.iter().any(|r| r.pending.id == id
            && r.state == WorkState::Done
            && r.delivery == Some(DeliveryOutcome::Accepted)));
        assert_eq!(
            records
                .iter()
                .filter(|r| r.state == WorkState::Stopped(StopReason::Denied))
                .count(),
            2
        );
        let rt = runtime(store, adapter, clock);
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        assert_eq!(stream.info().await.unwrap().state.messages, 1);
        rt.shutdown().await.unwrap();
    }
}
