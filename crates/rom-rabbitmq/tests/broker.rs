//! Required real RabbitMQ publisher-confirm and routing fixture.
use lapin::{
    Connection, ConnectionProperties,
    options::*,
    types::{AMQPValue, FieldTable},
};
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_rabbitmq::RabbitMqDelivery;
use std::time::Duration;
async fn connect() -> Connection {
    let uri = std::env::var("ROM_EXTRAS_RABBITMQ_URI").expect("required isolated RabbitMQ fixture");
    assert!(
        uri.starts_with("amqp://rom_extras:") && uri.ends_with("@127.0.0.1:55445/rom_extras"),
        "dedicated loopback fixture required"
    );
    tokio::time::timeout(
        Duration::from_secs(5),
        Connection::connect(&uri, ConnectionProperties::default()),
    )
    .await
    .expect("connect deadline")
    .unwrap_or_else(|_| panic!("fixture AMQP connection failed"))
}
fn quorum() -> FieldTable {
    let mut args = FieldTable::default();
    args.insert(
        "x-queue-type".into(),
        AMQPValue::LongString("quorum".into()),
    );
    args
}
#[tokio::test]
async fn confirms_persistence_return_and_redelivery_after_restart() {
    let connection = connect().await;
    let admin = connection.create_channel().await.unwrap();
    let name = format!(
        "ROMEXTRAS_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    admin
        .queue_declare(
            name.clone().into(),
            QueueDeclareOptions::durable(),
            quorum(),
        )
        .await
        .unwrap();
    let adapter = RabbitMqDelivery::bind(
        &connection,
        &name,
        Duration::from_secs(2),
        PayloadLimit::default(),
    )
    .await
    .unwrap();
    for attempt in [1, 2] {
        assert_eq!(
            adapter
                .deliver(Delivery {
                    id: "work-17".into(),
                    attempt,
                    payload: false
                })
                .await,
            DeliveryOutcome::Accepted
        );
    }
    let info = admin
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
    assert_eq!(info.message_count(), 2); // AMQP message_id does not deduplicate publications.
    let first = admin
        .basic_get(name.clone().into(), BasicGetOptions::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.data, b"false");
    assert_eq!(
        first.properties.message_id().as_ref().unwrap().as_str(),
        "work-17"
    );
    assert_eq!(first.properties.delivery_mode(), &Some(2));
    assert_eq!(
        first.properties.content_type().as_ref().unwrap().as_str(),
        "application/json"
    );
    drop(first); // Deliberately withhold consumer ack.
    let label = std::process::Command::new("docker")
        .args([
            "inspect",
            "rom-extras-rabbitmq-20261008",
            "--format",
            "{{index .Config.Labels \"rom-extras.fixture\"}}",
        ])
        .output()
        .unwrap();
    assert!(label.status.success());
    assert_eq!(String::from_utf8(label.stdout).unwrap().trim(), "rabbitmq");
    assert!(
        tokio::task::spawn_blocking(|| std::process::Command::new("docker")
            .args(["restart", "--time", "10", "rom-extras-rabbitmq-20261008"])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success())
        .await
        .unwrap()
    );
    // RabbitMQ starts asynchronously after the container process starts.
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let ready = tokio::task::spawn_blocking(|| {
                std::process::Command::new("docker")
                    .args([
                        "exec",
                        "rom-extras-rabbitmq-20261008",
                        "rabbitmq-diagnostics",
                        "-q",
                        "check_running",
                    ])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
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
    let reconnected = connect().await;
    let reader = reconnected.create_channel().await.unwrap();
    let info = reader
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
    assert_eq!(info.message_count(), 2);
    let first = reader
        .basic_get(name.clone().into(), BasicGetOptions::default())
        .await
        .unwrap()
        .unwrap();
    assert!(first.redelivered);
    assert_eq!(first.data, b"false");
    assert_eq!(
        first.properties.message_id().as_ref().unwrap().as_str(),
        "work-17"
    );
    first.ack(BasicAckOptions::default()).await.unwrap();
    let second = reader
        .basic_get(name.clone().into(), BasicGetOptions::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.data, b"false");
    second.ack(BasicAckOptions::default()).await.unwrap();
    let rebound = RabbitMqDelivery::bind(
        &reconnected,
        &name,
        Duration::from_secs(2),
        PayloadLimit::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        rebound
            .deliver(Delivery {
                id: "after-restart".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    let small = RabbitMqDelivery::bind(
        &reconnected,
        &name,
        Duration::from_secs(2),
        PayloadLimit::new(4).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        small
            .deliver(Delivery {
                id: "oversized".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Permanent
    );
    assert_eq!(
        rebound
            .deliver(Delivery {
                id: "x".repeat(256),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Permanent
    );
    let empty = format!("EMPTY_{name}");
    reader
        .queue_declare(
            empty.clone().into(),
            QueueDeclareOptions::durable(),
            quorum(),
        )
        .await
        .unwrap();
    let unrouted = RabbitMqDelivery::bind(
        &reconnected,
        &empty,
        Duration::from_secs(2),
        PayloadLimit::default(),
    )
    .await
    .unwrap();
    reader
        .queue_delete(empty.clone().into(), QueueDeleteOptions::default())
        .await
        .unwrap();
    assert_eq!(
        unrouted
            .deliver(Delivery {
                id: "unrouted".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Retryable
    );
    reader
        .queue_declare(
            empty.clone().into(),
            QueueDeclareOptions::durable(),
            quorum(),
        )
        .await
        .unwrap();
    assert_eq!(
        unrouted
            .deliver(Delivery {
                id: "unrouted".into(),
                attempt: 2,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    for invalid in ["", "amq.reserved", "bad\nname"] {
        assert!(
            RabbitMqDelivery::bind(
                &reconnected,
                invalid,
                Duration::from_secs(2),
                PayloadLimit::default()
            )
            .await
            .is_err()
        );
    }
    assert!(
        RabbitMqDelivery::bind(&reconnected, &name, Duration::ZERO, PayloadLimit::default())
            .await
            .is_err()
    );
    let info = reader
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
    assert_eq!(info.message_count(), 1);
    let timed = RabbitMqDelivery::bind(
        &reconnected,
        &name,
        Duration::from_millis(200),
        PayloadLimit::default(),
    )
    .await
    .unwrap();
    let cancelled = RabbitMqDelivery::bind(
        &reconnected,
        &name,
        Duration::from_secs(2),
        PayloadLimit::default(),
    )
    .await
    .unwrap();
    let paused = PausedBroker::new();
    let (unknown, overload) = tokio::join!(
        timed.deliver(Delivery {
            id: "timed-publication".into(),
            attempt: 1,
            payload: false
        }),
        timed.deliver(Delivery {
            id: "overloaded".into(),
            attempt: 1,
            payload: false
        })
    );
    assert_eq!(unknown, DeliveryOutcome::Unknown);
    assert_eq!(overload, DeliveryOutcome::Retryable);
    assert_eq!(
        timed
            .deliver(Delivery {
                id: "retired-refusal".into(),
                attempt: 2,
                payload: false
            })
            .await,
        DeliveryOutcome::Retryable
    );
    let mut flight = Box::pin(cancelled.deliver(Delivery {
        id: "cancelled-publication".into(),
        attempt: 1,
        payload: false,
    }));
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut flight)
            .await
            .is_err()
    );
    drop(flight);
    assert_eq!(
        cancelled
            .deliver(Delivery {
                id: "cancelled-refusal".into(),
                attempt: 2,
                payload: false
            })
            .await,
        DeliveryOutcome::Retryable
    );
    drop(paused);
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let info = reader
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
            if info.message_count() == 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("already-buffered timeout/cancel commands may publish later");
    assert_eq!(
        timed
            .deliver(Delivery {
                id: "retired-after-ack".into(),
                attempt: 3,
                payload: false
            })
            .await,
        DeliveryOutcome::Retryable
    );
    assert_eq!(
        cancelled
            .deliver(Delivery {
                id: "cancelled-after-ack".into(),
                attempt: 3,
                payload: false
            })
            .await,
        DeliveryOutcome::Retryable
    );
    let info = reader
        .queue_declare(
            name.into(),
            QueueDeclareOptions {
                passive: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();
    assert_eq!(info.message_count(), 3); // No overloaded or retired attempt was sent.
}

struct PausedBroker;
impl PausedBroker {
    fn new() -> Self {
        assert!(
            std::process::Command::new("docker")
                .args(["pause", "rom-extras-rabbitmq-20261008"])
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap()
                .success()
        );
        Self
    }
}
impl Drop for PausedBroker {
    fn drop(&mut self) {
        let result = std::process::Command::new("docker")
            .args(["unpause", "rom-extras-rabbitmq-20261008"])
            .stdout(std::process::Stdio::null())
            .status();
        assert!(
            result.is_ok_and(|s| s.success()),
            "restore dedicated broker after pause"
        );
    }
}
