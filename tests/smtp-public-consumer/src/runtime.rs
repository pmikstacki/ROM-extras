//! Independent native submission recovery through public ROM APIs only.
use rom::*;
use rom_smtp::Smtp;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Resource)]
#[resource(name = "smtp-notices")]
struct Notice {
    count: u64,
}
const CHANNEL: Channel<rom_email_core::EmailNotification> = Channel::new("smtp-notify", 1);
const NOTIFY: Action<Notice, rom_email_core::EmailNotification> =
    Action::new("notify", |state, message| {
        state.count += 1;
        Ok(vec![CHANNEL.intent(message)])
    });
struct TestClock(AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn service() -> Actor {
    Actor::trusted("fixture", "smtp").with_kind(PrincipalKind::Service)
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
fn runtime(store: Arc<dyn Storage>, adapter: Arc<Smtp>, clock: Arc<TestClock>) -> Runtime {
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
async fn enqueue(rt: &Runtime, id: &str, subject: &str) {
    rt.execute(
        &owner(),
        Command::create(id, Notice { count: 0 }).idempotency(&format!("seed-{id}")),
    )
    .await
    .unwrap();
    let payload = rom_email_core::EmailNotification::new(
        "Exact+Tag@example.invalid",
        subject,
        "Treść natywna\n.\n",
        1800000000,
    )
    .unwrap();
    rt.execute(
        &owner(),
        Command::action(id, NOTIFY, payload)
            .at_revision(1)
            .idempotency(&format!("notify-{id}")),
    )
    .await
    .unwrap();
}
pub(super) async fn qualify(ca: &[u8]) {
    let root = std::path::PathBuf::from(super::value("ROM_EXTRAS_SMTP_RUNTIME_PATH"));
    std::fs::create_dir_all(&root).unwrap();
    for redb in [false, true] {
        let backend = if redb { "redb" } else { "sqlite" };
        let path = root.join(backend);
        let subject = format!(
            "runtime-{}-{backend}",
            super::value("ROM_EXTRAS_SMTP_RUN_ID")
        );
        let adapter = Arc::new(
            Smtp::with_private_root(
                super::endpoint("mailpit.fixture.test"),
                super::credentials(super::value("ROM_EXTRAS_SMTP_PASSWORD")),
                super::profile(),
                super::limits(),
                ca,
            )
            .unwrap(),
        );
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let store = open(redb, &path);
        let rt = runtime(store.clone(), adapter.clone(), clock.clone());
        enqueue(&rt, "retry", &subject).await;
        let id = store.reaction_records().unwrap()[0].pending.id.clone();
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = open(redb, &path);
        let rt = runtime(store.clone(), adapter.clone(), clock.clone());
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
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
        enqueue(&rt, "source-denied", &format!("denied-source-{subject}")).await;
        rt.execute(
            &owner(),
            Command::replace("source-denied", Notice { count: 2 })
                .at_revision(2)
                .idempotency("revoke-source"),
        )
        .await
        .unwrap();
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
        enqueue(&rt, "service-denied", &format!("denied-service-{subject}")).await;
        rt.revoke(&service());
        assert_eq!(rt.process_work(1).await.unwrap(), 1);
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
        assert!(
            store
                .reaction_records()
                .unwrap()
                .iter()
                .any(|r| r.pending.id == id
                    && r.state == WorkState::Done
                    && r.delivery == Some(DeliveryOutcome::Accepted))
        );
        let rt = runtime(store, adapter, clock);
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        rt.shutdown().await.unwrap();
        println!(
            "Native SMTP Runtime {backend}: persisted Unknown, explicit retry and authority checks passed"
        );
    }
}
