//! Actual post-commit publisher-confirm loss, durable Runtime retry, and durable receiver idempotency.
mod common;
use common::{AckProxy, Receiver};
use lapin::{
    Connection, ConnectionProperties,
    options::*,
    types::{AMQPValue, FieldTable},
};
use rom::*;
use rom_delivery_core::PayloadLimit;
use rom_rabbitmq::RabbitMqDelivery;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
#[derive(Clone, Resource)]
#[resource(name = "rabbitmq-notices")]
struct Notice {
    count: u64,
}
const CHANNEL: Channel<bool> = Channel::new("extras-rabbitmq", 1);
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
    Actor::trusted("fixture", "rabbitmq").with_kind(PrincipalKind::Service)
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
    adapter: Arc<RabbitMqDelivery>,
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
async fn connect(uri: &str) -> Connection {
    tokio::time::timeout(
        Duration::from_secs(3),
        Connection::connect(uri, ConnectionProperties::default()),
    )
    .await
    .expect("fixture connection deadline")
    .unwrap_or_else(|_| panic!("fixture AMQP connection failed"))
}
#[tokio::test]
async fn reopened_runtime_retries_committed_message_and_receiver_deduplicates_durably() {
    let uri = std::env::var("ROM_EXTRAS_RABBITMQ_URI").expect("required RabbitMQ fixture");
    assert!(
        uri.starts_with("amqp://rom_extras:") && uri.ends_with("@127.0.0.1:55445/rom_extras"),
        "dedicated fixture required"
    );
    for redb in [false, true] {
        let direct = connect(&uri).await;
        let reader = direct.create_channel().await.unwrap();
        let name = format!(
            "RT_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let mut args = FieldTable::default();
        args.insert(
            "x-queue-type".into(),
            AMQPValue::LongString("quorum".into()),
        );
        reader
            .queue_declare(name.clone().into(), QueueDeclareOptions::durable(), args)
            .await
            .unwrap();
        let proxy = AckProxy::start(&name);
        let publisher = connect(&proxy.uri(&uri)).await;
        let adapter = Arc::new(
            RabbitMqDelivery::bind(
                &publisher,
                &name,
                Duration::from_secs(2),
                PayloadLimit::default(),
            )
            .await
            .unwrap(),
        );
        let path = std::env::temp_dir().join(format!("rom-rabbitmq-runtime-{name}"));
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
                .queue_declare(
                    name.clone().into(),
                    QueueDeclareOptions {
                        passive: true,
                        ..Default::default()
                    },
                    FieldTable::default()
                )
                .await
                .unwrap()
                .message_count(),
            0
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        let rt = runtime(store.clone(), adapter.clone(), clock.clone());
        let completed = AtomicBool::new(false);
        let processing = async {
            let result = rt.process_work(1).await;
            completed.store(true, Ordering::SeqCst);
            result
        };
        let witness = async {
            tokio::time::timeout(Duration::from_secs(1), async {
                while !proxy.dropped() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("actual publisher confirm must be dropped before deadline");
            let received = reader
                .basic_get(name.clone().into(), BasicGetOptions::default())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(received.data, b"false");
            assert_eq!(
                received.properties.message_id().as_ref().unwrap().as_str(),
                id
            );
            received
                .nack(BasicNackOptions {
                    requeue: true,
                    ..Default::default()
                })
                .await
                .unwrap();
            assert!(
                !completed.load(Ordering::SeqCst),
                "direct broker witness must complete while Runtime still awaits confirm"
            );
        };
        let (processed, ()) = tokio::join!(processing, witness);
        assert_eq!(processed.unwrap(), 1);
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
        // Host explicitly opens a fresh confirm channel. No adapter message retry is used.
        let rebound = Arc::new(
            RabbitMqDelivery::bind(
                &publisher,
                &name,
                Duration::from_secs(2),
                PayloadLimit::default(),
            )
            .await
            .unwrap(),
        );
        let rt = runtime(store.clone(), rebound, clock.clone());
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.pending.id, id);
        assert_eq!(record.attempts, 2);
        assert_eq!(record.state, WorkState::Done);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Accepted));
        assert_eq!(
            reader
                .queue_declare(
                    name.clone().into(),
                    QueueDeclareOptions {
                        passive: true,
                        ..Default::default()
                    },
                    FieldTable::default()
                )
                .await
                .unwrap()
                .message_count(),
            2
        );
        let receiver_path = std::env::temp_dir().join(format!("rom-rabbitmq-receiver-{name}"));
        let receiver = Receiver::open(redb, &receiver_path, true).await;
        let consumer_connection = connect(&uri).await;
        let consumer = consumer_connection.create_channel().await.unwrap();
        let first = consumer
            .basic_get(name.clone().into(), BasicGetOptions::default())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.properties.message_id().as_ref().unwrap().as_str(), id);
        receiver.accept(&id, &first.data).await;
        assert_eq!(receiver.count().await, 1);
        // Commit the effect, close the consumer without an ack, and reopen its Runtime.
        receiver.shutdown().await;
        drop(receiver);
        drop(first);
        consumer_connection
            .close(200, "fixture unacknowledged consumer".into())
            .await
            .unwrap();
        drop(consumer);
        drop(consumer_connection);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let queue = reader
                    .queue_declare(
                        name.clone().into(),
                        QueueDeclareOptions {
                            passive: true,
                            ..Default::default()
                        },
                        FieldTable::default(),
                    )
                    .await
                    .unwrap();
                if queue.message_count() == 2 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("broker automatically requeues the unacknowledged delivery");
        let receiver = Receiver::open(redb, &receiver_path, false).await;
        let mut redelivered = false;
        for _ in 0..2 {
            let received = reader
                .basic_get(name.clone().into(), BasicGetOptions::default())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                received.properties.message_id().as_ref().unwrap().as_str(),
                id
            );
            receiver.accept(&id, &received.data).await;
            redelivered |= received.redelivered;
            received.ack(BasicAckOptions::default()).await.unwrap();
        }
        assert!(
            redelivered,
            "connection closure caused actual broker redelivery"
        );
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
                .queue_declare(
                    name.clone().into(),
                    QueueDeclareOptions {
                        passive: true,
                        ..Default::default()
                    },
                    FieldTable::default()
                )
                .await
                .unwrap()
                .message_count(),
            0
        );
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
        let fresh = Arc::new(
            RabbitMqDelivery::bind(
                &publisher,
                &name,
                Duration::from_secs(2),
                PayloadLimit::default(),
            )
            .await
            .unwrap(),
        );
        let rt = runtime(store, fresh, clock);
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        rt.shutdown().await.unwrap();
    }
}
