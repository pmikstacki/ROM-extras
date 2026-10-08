//! Durable ROM channel integration against real HTTPS and native stores.
#![cfg(feature = "loopback-fixture")]
mod common;
use common::Receiver;
use rom::*;
use rom_webhook::TransportLimits;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Resource)]
#[resource(name = "webhook-notices")]
struct Notice {
    count: u64,
}
const WEBHOOK: Channel<bool> = Channel::new("extras-webhook", 1);
const NOTIFY: Action<Notice, ()> = Action::new("notify", |state, ()| {
    state.count += 1;
    Ok(vec![WEBHOOK.intent(false)])
});
struct TestClock(AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn service() -> Actor {
    Actor::trusted("fixture", "webhook").with_kind(PrincipalKind::Service)
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
    receiver: &Receiver,
    clock: Arc<TestClock>,
    endpoint: &str,
) -> Runtime {
    let transport = Arc::new(receiver.client(endpoint, TransportLimits::default()));
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
        .channel(WEBHOOK, service(), move |delivery| {
            let transport = transport.clone();
            async move { transport.deliver(delivery, 1_800_000_000).await }
        })
        .build(store, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
async fn enqueue(runtime: &Runtime, id: &str) {
    runtime
        .execute(
            &owner(),
            Command::create(id, Notice { count: 0 }).idempotency(&format!("seed-{id}")),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &owner(),
            Command::action(id, NOTIFY, ())
                .at_revision(1)
                .idempotency(&format!("notify-{id}")),
        )
        .await
        .unwrap();
}

// A callback bypass, lost intent, or non-persisted result breaks this integration.
#[tokio::test]
async fn durable_intent_reopen_delivery_and_revocation() {
    for redb in [false, true] {
        let receiver = Receiver::start();
        let path = std::env::temp_dir().join(format!(
            "rom-extras-webhook-runtime-{}-{redb}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let store = open(redb, &path);
        let rt = runtime(store.clone(), &receiver, clock.clone(), "/runtime-accept");
        enqueue(&rt, "accepted").await;
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.state, WorkState::Pending);
        let id = record.pending.id;
        assert_eq!(record.delivery, None);
        assert!(!receiver.stats().await.contains("/runtime-accept="));
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        let rt = runtime(store.clone(), &receiver, clock.clone(), "/runtime-accept");
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.pending.id, id);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Accepted));
        assert_eq!(record.state, WorkState::Done);
        enqueue(&rt, "revoked").await;
        rt.revoke(&service());
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        let records = store.reaction_records().unwrap();
        assert!(
            records
                .iter()
                .any(|r| r.state == WorkState::Stopped(StopReason::Denied))
        );
        assert!(
            receiver
                .stats()
                .await
                .lines()
                .any(|line| line == "durable-path:/runtime-accept=1")
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        let records = store.reaction_records().unwrap();
        assert!(records.iter().any(|r| r.pending.id == id
            && r.state == WorkState::Done
            && r.delivery == Some(DeliveryOutcome::Accepted)));
        assert!(
            records
                .iter()
                .any(|r| r.state == WorkState::Stopped(StopReason::Denied))
        );
    }
}

#[tokio::test]
async fn lost_ack_retry_survives_receiver_restart_and_attempt_budget_is_finite() {
    for redb in [false, true] {
        let mut receiver = Receiver::start();
        let path = std::env::temp_dir().join(format!(
            "rom-extras-webhook-retry-{}-{redb}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let store = open(redb, &path);
        let rt = runtime(store.clone(), &receiver, clock.clone(), "/runtime-lost-ack");
        enqueue(&rt, "retry").await;
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        let record = store.reaction_records().unwrap().remove(0);
        let id = record.pending.id;
        assert_eq!(record.delivery, Some(DeliveryOutcome::Unknown));
        assert_eq!(record.state, WorkState::Pending);
        assert_eq!(record.attempts, 1);
        assert_eq!(rt.process_work(1).await.unwrap(), 0);
        assert!(
            receiver
                .stats()
                .await
                .lines()
                .any(|line| line == "durable-effects=1")
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        receiver.restart();
        let store = open(redb, &path);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Unknown));
        assert_eq!(record.pending.id, id);
        clock.0.store(1, Ordering::SeqCst);
        let rt = runtime(store.clone(), &receiver, clock.clone(), "/runtime-lost-ack");
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.pending.id, id);
        assert_eq!(record.attempts, 2);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Accepted));
        assert_eq!(record.state, WorkState::Done);
        let stats = receiver.stats().await;
        for line in [
            "durable-effects=1".to_owned(),
            "durable-path:/runtime-lost-ack=2".to_owned(),
            format!("durable-id:{id}=ZmFsc2U="),
        ] {
            assert!(stats.lines().any(|actual| actual == line), "{stats}");
        }
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        let rt = runtime(
            store.clone(),
            &receiver,
            clock.clone(),
            "/runtime-always-disconnect",
        );
        enqueue(&rt, "bounded").await;
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        assert_eq!(rt.process_work(1).await.unwrap(), 0);
        clock.0.store(2, Ordering::SeqCst);
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        let records = store.reaction_records().unwrap();
        let bounded = records.iter().find(|r| r.pending.id != id).unwrap();
        assert_eq!(bounded.state, WorkState::Stopped(StopReason::Attempts));
        assert_eq!(bounded.attempts, 2);
        assert_eq!(bounded.delivery, Some(DeliveryOutcome::Unknown));
        clock.0.store(100, Ordering::SeqCst);
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        let stats = receiver.stats().await;
        assert!(
            stats
                .lines()
                .any(|line| line == "durable-path:/runtime-always-disconnect=2")
        );
        assert!(stats.lines().any(|line| line == "durable-effects=2"));
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        assert!(
            store
                .reaction_records()
                .unwrap()
                .iter()
                .any(|r| r.state == WorkState::Stopped(StopReason::Attempts)
                    && r.delivery == Some(DeliveryOutcome::Unknown))
        );
        let rt = runtime(
            store.clone(),
            &receiver,
            clock,
            "/runtime-always-disconnect",
        );
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        assert!(
            receiver
                .stats()
                .await
                .lines()
                .any(|line| line == "durable-path:/runtime-always-disconnect=2")
        );
        rt.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn source_disclosure_revocation_stops_pending_payload_before_transport() {
    for redb in [false, true] {
        let receiver = Receiver::start();
        let path = std::env::temp_dir().join(format!(
            "rom-extras-webhook-source-denied-{}-{redb}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let store = open(redb, &path);
        let rt = runtime(store.clone(), &receiver, clock, "/runtime-accept");
        enqueue(&rt, "source-denied").await;
        rt.execute(
            &owner(),
            Command::replace("source-denied", Notice { count: 2 })
                .at_revision(2)
                .idempotency("source-visibility"),
        )
        .await
        .unwrap();
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        let record = store.reaction_records().unwrap().remove(0);
        assert_eq!(record.state, WorkState::Stopped(StopReason::Denied));
        assert_eq!(record.delivery, None);
        let stats = receiver.stats().await;
        assert!(stats.lines().any(|line| line == "durable-effects=0"));
        assert!(!stats.contains("/runtime-accept="));
        rt.shutdown().await.unwrap();
    }
}
