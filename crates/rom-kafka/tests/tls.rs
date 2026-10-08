//! Direct verified TLS/SASL, isolated negative causes, and explicit reconnect after restart.
#[path = "../../../tests/common/kafka_tls_config.rs"]
mod tls_config;
use rdkafka::{
    ClientConfig, Message, Offset, TopicPartitionList,
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    client::{ClientContext, DefaultClientContext},
    config::RDKafkaLogLevel,
    consumer::{BaseConsumer, Consumer},
    error::KafkaError,
    message::Headers,
    producer::{FutureProducer, FutureRecord},
    types::RDKafkaErrorCode,
};
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_kafka::KafkaDelivery;
use std::{
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, Instant},
};
struct Causes(Arc<AtomicU8>);
impl ClientContext for Causes {
    fn log(&self, _: RDKafkaLogLevel, _: &str, _: &str) {}
    fn error(&self, error: KafkaError, _: &str) {
        let bit = match error.rdkafka_error_code() {
            Some(RDKafkaErrorCode::SSL) => 1,
            Some(RDKafkaErrorCode::Authentication | RDKafkaErrorCode::SaslAuthenticationFailed) => {
                2
            }
            Some(RDKafkaErrorCode::BrokerTransportFailure) => 4,
            _ => 0,
        };
        self.0.fetch_or(bit, Ordering::SeqCst);
    }
}

fn delivery(id: &str) -> Delivery<bool> {
    Delivery {
        id: id.into(),
        attempt: 1,
        payload: false,
    }
}
async fn denied(mut cfg: ClientConfig, topic: &str, id: &str, expected: u8) {
    let adapter = KafkaDelivery::new(
        cfg.clone(),
        topic,
        Duration::from_millis(1500),
        PayloadLimit::default(),
    )
    .unwrap_or_else(|_| panic!("negative adapter construction"));
    assert_eq!(
        adapter.deliver(delivery(id)).await,
        DeliveryOutcome::Unknown
    );
    cfg.set("acks", "all")
        .set("retries", "0")
        .set("message.timeout.ms", "1500")
        .set("socket.timeout.ms", "1500")
        .set("allow.auto.create.topics", "false");
    let causes = Arc::new(AtomicU8::new(0));
    let producer: FutureProducer<Causes> = cfg
        .create_with_context(Causes(causes.clone()))
        .unwrap_or_else(|_| panic!("native negative producer construction"));
    let queued = producer
        .send_result(FutureRecord::to(topic).key(id).payload("false"))
        .unwrap_or_else(|_| panic!("native negative enqueue"));
    let result = tokio::time::timeout(Duration::from_secs(3), queued)
        .await
        .expect("bounded native report");
    assert!(result.is_ok_and(|outcome| outcome.is_err()));
    assert_ne!(
        causes.load(Ordering::SeqCst) & expected,
        0,
        "allowlisted cause for {id}"
    );
}
fn tls_reader(topic: &str) -> BaseConsumer {
    let mut cfg = tls_config::config();
    cfg.set("group.id", topic)
        .set("enable.auto.commit", "false")
        .set("enable.auto.offset.store", "false")
        .set("allow.auto.create.topics", "false");
    let reader: BaseConsumer = cfg
        .create()
        .unwrap_or_else(|_| panic!("TLS reader construction"));
    let mut assignment = TopicPartitionList::new();
    assignment
        .add_partition_offset(topic, 0, Offset::Beginning)
        .unwrap();
    reader.assign(&assignment).unwrap();
    reader
}
fn read_exact(reader: &BaseConsumer, id: &str, offset: i64) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(message) = reader.poll(Duration::from_millis(50)) {
            let message = message.unwrap_or_else(|_| panic!("TLS read failure"));
            assert_eq!(message.offset(), offset);
            assert_eq!(message.key(), Some(id.as_bytes()));
            assert_eq!(message.payload(), Some(b"false".as_slice()));
            assert_eq!(
                message.headers().unwrap().get(0).value,
                Some(b"application/json".as_slice())
            );
            return;
        }
        assert!(Instant::now() < deadline, "bounded TLS read");
    }
}
fn ready(topic: &str) {
    let probe: BaseConsumer = tls_config::config()
        .create()
        .unwrap_or_else(|_| panic!("TLS readiness client"));
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if probe
            .fetch_metadata(Some(topic), Duration::from_millis(300))
            .is_ok_and(|metadata| {
                metadata.topics().iter().any(|entry| {
                    entry.name() == topic
                        && entry.error().is_none()
                        && entry.partitions().iter().any(|partition| {
                            partition.id() == 0
                                && partition.leader() == 1
                                && partition.error().is_none()
                        })
                })
            })
        {
            return;
        }
        assert!(Instant::now() < deadline, "bounded TLS topic readiness");
        std::thread::sleep(Duration::from_millis(50));
    }
}
#[tokio::test]
async fn verified_sasl_tls_rejects_bad_peers_and_reconnects_after_restart() {
    let topic = format!(
        "ROMEXTRAS_TLS_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let cfg = tls_config::config();
    let admin: AdminClient<DefaultClientContext> = cfg
        .create()
        .unwrap_or_else(|_| panic!("TLS admin construction"));
    let provisioned = tokio::time::timeout(
        Duration::from_secs(5),
        admin.create_topics(
            &[NewTopic::new(&topic, 1, TopicReplication::Fixed(1))],
            &AdminOptions::new(),
        ),
    )
    .await
    .expect("bounded TLS provisioning")
    .unwrap_or_else(|_| panic!("TLS admin request"));
    assert!(provisioned.into_iter().all(|result| result.is_ok()));
    ready(&topic);
    let adapter = KafkaDelivery::new(
        cfg.clone(),
        &topic,
        Duration::from_secs(3),
        PayloadLimit::default(),
    )
    .unwrap();
    assert_eq!(
        adapter.deliver(delivery("tls-first-17")).await,
        DeliveryOutcome::Accepted
    );
    let reader = tls_reader(&topic);
    read_exact(&reader, "tls-first-17", 0);
    let mut wrong_ca = cfg.clone();
    wrong_ca.set(
        "ssl.ca.location",
        std::env::var("ROM_EXTRAS_KAFKA_TLS_WRONG_CA").expect("wrong CA required"),
    );
    denied(wrong_ca, &topic, "wrong-ca", 1).await;
    let mut wrong_name = cfg.clone();
    wrong_name.set("bootstrap.servers", "127.0.0.1:55450");
    denied(wrong_name, &topic, "wrong-name", 1).await;
    let mut wrong_password = cfg.clone();
    wrong_password.set("sasl.password", "intentionally-wrong");
    denied(wrong_password, &topic, "wrong-password", 2).await;
    let mut plain = cfg.clone();
    let mut plain_probe_cfg = ClientConfig::new();
    plain_probe_cfg.set("bootstrap.servers", "127.0.0.1:55449");
    let plain_probe: BaseConsumer = plain_probe_cfg
        .create()
        .unwrap_or_else(|_| panic!("plaintext readiness client"));
    let metadata = plain_probe
        .fetch_metadata(None, Duration::from_secs(3))
        .expect("plaintext Kafka peer must answer metadata");
    assert!(
        metadata
            .brokers()
            .iter()
            .any(|broker| broker.host() == "127.0.0.1" && broker.port() == 55449)
    );
    plain
        .set("security.protocol", "ssl")
        .set("bootstrap.servers", "127.0.0.1:55449");
    plain.remove("sasl.username");
    plain.remove("sasl.password");
    plain.remove("sasl.mechanism");
    denied(plain, &topic, "tls-on-plain", 4).await;
    assert_eq!(
        reader
            .fetch_watermarks(&topic, 0, Duration::from_secs(3))
            .unwrap(),
        (0, 1)
    );
    drop(reader);
    drop(adapter);
    drop(admin);
    tokio::task::spawn_blocking(|| {
        let label = Command::new("docker")
            .args([
                "inspect",
                "rom-extras-kafka-tls-20261008",
                "--format",
                "{{index .Config.Labels \"rom-extras.fixture\"}}",
            ])
            .output()
            .unwrap();
        assert!(label.status.success());
        assert_eq!(String::from_utf8(label.stdout).unwrap().trim(), "kafka-tls");
        assert!(
            Command::new("docker")
                .args(["restart", "--time", "10", "rom-extras-kafka-tls-20261008"])
                .stdout(Stdio::null())
                .status()
                .unwrap()
                .success()
        );
    })
    .await
    .unwrap();
    ready(&topic);
    let reader = tls_reader(&topic);
    read_exact(&reader, "tls-first-17", 0);
    let rebound = KafkaDelivery::new(
        tls_config::config(),
        &topic,
        Duration::from_secs(3),
        PayloadLimit::default(),
    )
    .unwrap();
    assert_eq!(
        rebound.deliver(delivery("tls-rebound-18")).await,
        DeliveryOutcome::Accepted
    );
    read_exact(&reader, "tls-rebound-18", 1);
    assert_eq!(
        reader
            .fetch_watermarks(&topic, 0, Duration::from_secs(3))
            .unwrap(),
        (0, 2)
    );
}
