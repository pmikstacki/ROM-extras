//! Dedicated Kafka fixture operations shared by required test targets.
use rdkafka::{
    ClientConfig, Offset, TopicPartitionList,
    consumer::{BaseConsumer, Consumer},
    message::OwnedMessage,
};
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};
pub fn config() -> ClientConfig {
    let brokers = std::env::var("ROM_EXTRAS_KAFKA_BROKERS").expect("required Kafka fixture");
    assert_eq!(brokers, "127.0.0.1:55449");
    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", brokers);
    config
}
pub fn control(args: &[&str]) {
    let label = Command::new("docker")
        .args([
            "inspect",
            "rom-extras-kafka-20261008",
            "--format",
            "{{index .Config.Labels \"rom-extras.fixture\"}}",
        ])
        .output()
        .unwrap();
    assert!(label.status.success());
    assert_eq!(String::from_utf8(label.stdout).unwrap().trim(), "kafka");
    assert!(
        Command::new("docker")
            .args(args)
            .stdout(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
}
pub fn provision(topic: &str) {
    control(&[
        "exec",
        "rom-extras-kafka-20261008",
        "/opt/kafka/bin/kafka-topics.sh",
        "--bootstrap-server",
        "127.0.0.1:19092",
        "--create",
        "--topic",
        topic,
        "--partitions",
        "1",
        "--replication-factor",
        "1",
    ]);
}
pub fn consumer(topic: &str) -> BaseConsumer {
    let mut cfg = config();
    cfg.set("group.id", topic)
        .set("enable.auto.commit", "false")
        .set("enable.auto.offset.store", "false")
        .set("allow.auto.create.topics", "false");
    let consumer: BaseConsumer = cfg.create().unwrap();
    let mut assigned = TopicPartitionList::new();
    assigned
        .add_partition_offset(topic, 0, Offset::Beginning)
        .unwrap();
    consumer.assign(&assigned).unwrap();
    consumer
}
pub fn read(consumer: &BaseConsumer) -> OwnedMessage {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(message) = consumer.poll(Duration::from_millis(50)) {
            return message.expect("real Kafka read").detach();
        }
        assert!(Instant::now() < deadline, "bounded consumer read");
    }
}
