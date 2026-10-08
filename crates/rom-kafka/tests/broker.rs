//! Required single-node Kafka acceptance, persistence and bounded uncertainty.
mod common;
use common::fixture::{config, consumer, control, provision, read};
use rdkafka::{Message, message::Headers};
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_kafka::KafkaDelivery;
use std::{
    process::{Command, Stdio},
    time::Duration,
};
fn message(id: &str, attempt: u32) -> Delivery<bool> {
    Delivery {
        id: id.into(),
        attempt,
        payload: false,
    }
}
struct Paused;
impl Paused {
    fn start() -> Self {
        control(&["pause", "rom-extras-kafka-20261008"]);
        Self
    }
}
impl Drop for Paused {
    fn drop(&mut self) {
        control(&["unpause", "rom-extras-kafka-20261008"]);
    }
}
#[tokio::test]
async fn real_offsets_keys_duplicates_restart_and_cancelled_publication() {
    assert_eq!(rdkafka::util::get_rdkafka_version().1, "2.12.1");
    let name = format!(
        "ROMEXTRAS_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    provision(&name);
    let adapter = KafkaDelivery::new(
        config(),
        &name,
        Duration::from_secs(3),
        PayloadLimit::default(),
    )
    .unwrap();
    for attempt in [1, 2] {
        assert_eq!(
            adapter.deliver(message("work-17", attempt)).await,
            DeliveryOutcome::Accepted
        );
    }
    let reader = consumer(&name);
    for offset in [0, 1] {
        let received = read(&reader);
        assert_eq!(received.offset(), offset);
        assert_eq!(received.partition(), 0);
        assert_eq!(received.key(), Some(b"work-17".as_slice()));
        assert_eq!(received.payload(), Some(b"false".as_slice()));
        let header = received.headers().unwrap().get(0);
        assert_eq!(header.key, "content-type");
        assert_eq!(header.value, Some(b"application/json".as_slice()));
    }
    drop(reader);
    tokio::task::spawn_blocking(|| {
        control(&["restart", "--time", "10", "rom-extras-kafka-20261008"])
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let ready = tokio::task::spawn_blocking(|| {
                Command::new("docker")
                    .args([
                        "exec",
                        "rom-extras-kafka-20261008",
                        "/opt/kafka/bin/kafka-topics.sh",
                        "--bootstrap-server",
                        "127.0.0.1:19092",
                        "--list",
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            })
            .await
            .unwrap();
            if ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    let reader = consumer(&name);
    for offset in [0, 1] {
        let received = read(&reader);
        assert_eq!(received.offset(), offset);
        assert_eq!(received.key(), Some(b"work-17".as_slice()));
        assert_eq!(received.payload(), Some(b"false".as_slice()));
    }
    let fresh = KafkaDelivery::new(
        config(),
        &name,
        Duration::from_secs(3),
        PayloadLimit::default(),
    )
    .unwrap();
    assert_eq!(
        fresh.deliver(message("warm-18", 1)).await,
        DeliveryOutcome::Accepted
    );
    let paused = Paused::start();
    let mut pending = Box::pin(fresh.deliver(message("cancel-19", 1)));
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut pending)
            .await
            .is_err()
    );
    assert_eq!(
        fresh.deliver(message("busy-must-not-publish", 1)).await,
        DeliveryOutcome::Retryable
    );
    drop(pending);
    assert_eq!(
        fresh.deliver(message("must-not-publish", 1)).await,
        DeliveryOutcome::Retryable
    );
    drop(paused);
    for id in ["warm-18", "cancel-19"] {
        let received = read(&reader);
        assert_eq!(received.key(), Some(id.as_bytes()));
        assert_eq!(received.payload(), Some(b"false".as_slice()));
    }
    assert!(reader.poll(Duration::from_millis(200)).is_none());
    let absent = format!("{name}_ABSENT");
    let unknown = KafkaDelivery::new(
        config(),
        &absent,
        Duration::from_millis(200),
        PayloadLimit::default(),
    )
    .unwrap();
    assert_eq!(
        unknown.deliver(message("unknown-20", 1)).await,
        DeliveryOutcome::Unknown
    );
    assert_eq!(
        unknown.deliver(message("unknown-20", 2)).await,
        DeliveryOutcome::Retryable
    );
    let listed = Command::new("docker")
        .args([
            "exec",
            "rom-extras-kafka-20261008",
            "/opt/kafka/bin/kafka-topics.sh",
            "--bootstrap-server",
            "127.0.0.1:19092",
            "--list",
        ])
        .output()
        .unwrap();
    assert!(listed.status.success());
    assert!(
        !String::from_utf8(listed.stdout)
            .unwrap()
            .lines()
            .any(|topic| topic == absent)
    );
    let small = KafkaDelivery::new(
        config(),
        &name,
        Duration::from_secs(2),
        PayloadLimit::new(4).unwrap(),
    )
    .unwrap();
    assert_eq!(
        small.deliver(message("oversize", 1)).await,
        DeliveryOutcome::Permanent
    );
    assert_eq!(
        small.deliver(message("invalid.id", 1)).await,
        DeliveryOutcome::Permanent
    );
    assert!(reader.poll(Duration::from_millis(200)).is_none());
    assert!(
        KafkaDelivery::new(
            config(),
            "..",
            Duration::from_secs(2),
            PayloadLimit::default()
        )
        .is_err()
    );
}
