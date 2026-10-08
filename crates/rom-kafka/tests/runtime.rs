//! Durable Runtime delivery recovery and receipt replay across native storage reopen.
#[path = "common/ack_proxy.rs"]
mod ack_proxy;
mod common;
#[path = "../../../tests/common/receipt_receiver.rs"]
mod receipt_receiver;
use ack_proxy::AckProxy;
use common::fixture::{config, consumer, provision, read};
use rdkafka::{Message, consumer::Consumer};
use receipt_receiver::Receiver;
use rom::*;
use rom_delivery_core::PayloadLimit;
use rom_kafka::KafkaDelivery;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
#[derive(Clone, Resource)]
#[resource(name = "kafka-notices")]
struct Notice {
    count: u64,
}
const CHANNEL: Channel<bool> = Channel::new("extras-kafka", 1);
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
    Actor::trusted("fixture", "kafka").with_kind(PrincipalKind::Service)
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
fn runtime(store: Arc<dyn Storage>, adapter: Arc<KafkaDelivery>, clock: Arc<TestClock>) -> Runtime {
    Runtime::builder()
        .delivery_timeout(Duration::from_secs(15))
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
async fn reopened_runtime_retries_same_identity_and_receiver_replays_durable_receipt() {
    for redb in [false, true] {
        let topic = format!(
            "ROMEXTRAS_RT_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        provision(&topic);
        let reader = consumer(&topic);
        let proxy = AckProxy::start(&topic);
        let mut cfg = config();
        cfg.set("bootstrap.servers", format!("127.0.0.1:{}", proxy.port));
        let adapter = Arc::new(
            KafkaDelivery::new(
                cfg,
                &topic,
                Duration::from_secs(10),
                PayloadLimit::default(),
            )
            .unwrap(),
        );
        let path = std::env::temp_dir().join(format!("rom-kafka-runtime-{topic}"));
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let store = open(redb, &path);
        let rt = runtime(store.clone(), adapter.clone(), clock.clone());
        enqueue(&rt, "retry").await;
        let record = store.reaction_records().unwrap().remove(0);
        let id = record.pending.id;
        assert_eq!(record.state, WorkState::Pending);
        assert_eq!(record.delivery, None);
        assert_eq!(
            reader
                .fetch_watermarks(&topic, 0, Duration::from_secs(3))
                .unwrap(),
            (0, 0)
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        let rt = runtime(store.clone(), adapter.clone(), clock.clone());
        let mut processing = Box::pin(rt.process_work(1));
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                tokio::select! {
                    result = &mut processing => panic!("Runtime completed before broker witness: {result:?}"),
                    () = tokio::time::sleep(Duration::from_millis(10)) => {}
                }
                if proxy.dropped() { break; }
            }
        }).await.expect("actual successful Kafka acknowledgement dropped");
        let received = read(&reader);
        assert_eq!(received.offset(), 0);
        assert_eq!(received.key(), Some(id.as_bytes()));
        assert_eq!(received.payload(), Some(b"false".as_slice()));
        assert!(
            tokio::time::timeout(Duration::from_millis(1), &mut processing)
                .await
                .is_err(),
            "broker witness precedes Runtime outcome"
        );
        assert_eq!(processing.await.unwrap(), 1);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.pending.id, id);
        assert_eq!(record.state, WorkState::Pending);
        assert_eq!(record.attempts, 1);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Unknown));
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
        let rebound = Arc::new(
            KafkaDelivery::new(
                config(),
                &topic,
                Duration::from_secs(3),
                PayloadLimit::default(),
            )
            .unwrap(),
        );
        let rt = runtime(store.clone(), rebound, clock.clone());
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.pending.id, id);
        assert_eq!(record.attempts, 2);
        assert_eq!(record.state, WorkState::Done);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Accepted));
        let repeated = read(&reader);
        assert_eq!(repeated.offset(), 1);
        assert_eq!(repeated.key(), Some(id.as_bytes()));
        assert_eq!(repeated.payload(), Some(b"false".as_slice()));
        assert_eq!(
            reader
                .fetch_watermarks(&topic, 0, Duration::from_secs(3))
                .unwrap(),
            (0, 2)
        );

        let receiver_path = std::env::temp_dir().join(format!("rom-kafka-receiver-{topic}"));
        let receiver = Receiver::open(redb, &receiver_path, true).await;
        let replay_reader = consumer(&topic);
        let first = read(&replay_reader);
        receiver.accept(&id, first.payload().unwrap()).await;
        assert_eq!(receiver.count().await, 1);
        // Durable effect committed; deliberately do not commit the Kafka consumer offset.
        receiver.shutdown().await;
        drop(receiver);
        drop(first);
        drop(replay_reader);
        let receiver = Receiver::open(redb, &receiver_path, false).await;
        let replay_reader = consumer(&topic);
        for offset in [0, 1] {
            let replay = read(&replay_reader);
            assert_eq!(replay.offset(), offset);
            assert_eq!(replay.key(), Some(id.as_bytes()));
            receiver.accept(&id, replay.payload().unwrap()).await;
        }
        assert_eq!(receiver.count().await, 1);
        receiver.shutdown().await;
        drop(receiver);
        let receiver = Receiver::open(redb, &receiver_path, false).await;
        assert_eq!(receiver.count().await, 1);
        receiver.shutdown().await;

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
        assert_eq!(
            reader
                .fetch_watermarks(&topic, 0, Duration::from_secs(3))
                .unwrap(),
            (0, 2)
        );
        assert_eq!(
            store
                .reaction_records()
                .unwrap()
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
        assert_eq!(
            store
                .reaction_records()
                .unwrap()
                .iter()
                .filter(
                    |r| r.state == WorkState::Stopped(StopReason::Denied) && r.delivery.is_none()
                )
                .count(),
            2
        );
        let fresh = Arc::new(
            KafkaDelivery::new(
                config(),
                &topic,
                Duration::from_secs(3),
                PayloadLimit::default(),
            )
            .unwrap(),
        );
        let rt = runtime(store, fresh, clock);
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        rt.shutdown().await.unwrap();
        assert!(reader.poll(Duration::from_millis(200)).is_none());
    }
}

#[tokio::test]
async fn native_unknown_attempt_budget_stops_durably_without_creating_topic() {
    for redb in [false, true] {
        let topic = format!(
            "ROMEXTRAS_MISSING_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(format!("rom-kafka-budget-{topic}"));
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let mut identity = None;
        for attempt in [1, 2] {
            clock.0.store(u64::from(attempt - 1), Ordering::SeqCst);
            let store = open(redb, &path);
            let adapter = Arc::new(
                KafkaDelivery::new(
                    config(),
                    &topic,
                    Duration::from_millis(200),
                    PayloadLimit::default(),
                )
                .unwrap(),
            );
            let rt = runtime(store.clone(), adapter, clock.clone());
            if attempt == 1 {
                enqueue(&rt, "bounded").await;
            }
            assert_eq!(rt.process_work(1).await.unwrap(), 1);
            let record = store.reaction_records().unwrap().remove(0);
            let id = identity.get_or_insert_with(|| record.pending.id.clone());
            assert_eq!(record.pending.id, *id);
            assert_eq!(record.attempts, attempt);
            assert_eq!(record.delivery, Some(DeliveryOutcome::Unknown));
            assert_eq!(
                record.state,
                if attempt == 1 {
                    WorkState::Pending
                } else {
                    WorkState::Stopped(StopReason::Attempts)
                }
            );
            assert_eq!(rt.process_work(1).await.unwrap(), 0);
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(store);
        }
        let store = open(redb, &path);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.state, WorkState::Stopped(StopReason::Attempts));
        assert_eq!(record.attempts, 2);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Unknown));
        clock.0.store(60, Ordering::SeqCst);
        let adapter = Arc::new(
            KafkaDelivery::new(
                config(),
                &topic,
                Duration::from_millis(200),
                PayloadLimit::default(),
            )
            .unwrap(),
        );
        let rt = runtime(store, adapter, clock);
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        rt.shutdown().await.unwrap();
        let reader = consumer(&topic);
        let metadata = reader.fetch_metadata(None, Duration::from_secs(3)).unwrap();
        assert!(!metadata.topics().iter().any(|entry| entry.name() == topic));
    }
}
