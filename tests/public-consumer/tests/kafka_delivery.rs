//! Kafka consumed through independent public construction and transport contracts.
#[path = "../../common/kafka_tls_config.rs"]
mod tls_config;
use rdkafka::{
    ClientConfig, Message, Offset, TopicPartitionList,
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    client::DefaultClientContext,
    consumer::{BaseConsumer, Consumer},
    message::Headers,
};
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_kafka::KafkaDelivery;
use std::time::{Duration, Instant};
#[tokio::test]
async fn external_consumer_receives_acknowledged_false_json_with_stable_key() {
    let brokers = std::env::var("ROM_EXTRAS_KAFKA_BROKERS").expect("required Kafka fixture");
    assert_eq!(brokers, "127.0.0.1:55449");
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", brokers);
    exercise(cfg).await;
}
#[tokio::test]
async fn external_consumer_receives_verified_sasl_tls_publication() {
    exercise(tls_config::config()).await;
}
async fn exercise(mut cfg: ClientConfig) {
    let admin: AdminClient<DefaultClientContext> = cfg.create().unwrap();
    let name = format!(
        "CONSUMER_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        admin.create_topics(
            &[NewTopic::new(&name, 1, TopicReplication::Fixed(1))],
            &AdminOptions::new(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(result.into_iter().all(|result| result.is_ok()));
    let adapter = KafkaDelivery::new(
        cfg.clone(),
        &name,
        Duration::from_secs(3),
        PayloadLimit::default(),
    )
    .unwrap();
    assert_eq!(
        adapter
            .deliver(Delivery {
                id: "external-consumer-17".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    cfg.set("group.id", &name)
        .set("enable.auto.commit", "false")
        .set("allow.auto.create.topics", "false");
    let reader: BaseConsumer = cfg.create().unwrap();
    let mut assigned = TopicPartitionList::new();
    assigned
        .add_partition_offset(&name, 0, Offset::Beginning)
        .unwrap();
    reader.assign(&assigned).unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(received) = reader.poll(Duration::from_millis(50)) {
            let received = received.unwrap();
            assert_eq!(received.offset(), 0);
            assert_eq!(received.key(), Some(b"external-consumer-17".as_slice()));
            assert_eq!(received.payload(), Some(b"false".as_slice()));
            assert_eq!(
                received.headers().unwrap().get(0).value,
                Some(b"application/json".as_slice())
            );
            break;
        }
        assert!(Instant::now() < deadline, "bounded independent Kafka read");
    }
}
