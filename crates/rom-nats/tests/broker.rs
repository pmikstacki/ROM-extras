//! Required real JetStream fixture; missing configuration fails instead of skipping.
use async_nats::jetstream::consumer::{AckPolicy, PullConsumer, pull};
use async_nats::jetstream::{
    self,
    stream::{Config, StorageType},
};
use futures_util::StreamExt;
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_nats::JetStreamDelivery;
use std::time::Duration;

#[tokio::test]
async fn acknowledges_exact_payload_and_deduplicates_after_broker_restart() {
    let url =
        std::env::var("ROM_EXTRAS_NATS_URL").expect("required isolated NATS fixture configuration");
    assert_eq!(
        url, "nats://127.0.0.1:55441",
        "restart test requires the dedicated loopback fixture"
    );
    let client = async_nats::ConnectOptions::with_token(
        std::env::var("ROM_EXTRAS_NATS_TOKEN").expect("required fixture token"),
    )
    .connection_timeout(Duration::from_secs(2))
    .connect(url)
    .await
    .unwrap_or_else(|_| panic!("fixture connection failed"));
    let context = jetstream::new(client);
    let name = format!(
        "ROMEXTRAS_{}_{}",
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
            max_bytes: 1_048_576,
            max_message_size: 65_536,
            duplicate_window: Duration::from_secs(120),
            ..Default::default()
        })
        .await
        .expect("create fixture stream");
    let adapter = JetStreamDelivery::bind(
        context.clone(),
        &name,
        &subject,
        1,
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
    assert_eq!(stream.info().await.unwrap().state.messages, 1);
    assert_eq!(
        stream.get_raw_message(1).await.unwrap().payload.as_ref(),
        b"false"
    );
    let consumer: PullConsumer = stream
        .create_consumer(pull::Config {
            durable_name: Some("rom-extras-reader".into()),
            ack_policy: AckPolicy::Explicit,
            ack_wait: Duration::from_millis(200),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut messages = consumer
        .fetch()
        .max_messages(1)
        .expires(Duration::from_secs(2))
        .messages()
        .await
        .unwrap();
    let first = tokio::time::timeout(Duration::from_secs(3), messages.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(first.payload.as_ref(), b"false");
    assert_eq!(
        first
            .headers
            .as_ref()
            .unwrap()
            .get("Nats-Msg-Id")
            .unwrap()
            .as_str(),
        "work-17"
    );
    assert_eq!(first.info().unwrap().stream_sequence, 1);
    assert_eq!(first.info().unwrap().delivered, 1);
    drop(first);
    drop(messages);
    drop(consumer); // Deliberately withhold consumer acknowledgement.
    let label = std::process::Command::new("docker")
        .args([
            "inspect",
            "rom-extras-nats-20261008",
            "--format",
            "{{index .Config.Labels \"rom-extras.fixture\"}}",
        ])
        .output()
        .unwrap();
    assert!(label.status.success());
    assert_eq!(String::from_utf8(label.stdout).unwrap().trim(), "nats");
    assert!(
        std::process::Command::new("docker")
            .args(["restart", "--time", "10", "rom-extras-nats-20261008"])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    let url = std::env::var("ROM_EXTRAS_NATS_URL").unwrap();
    let client = async_nats::ConnectOptions::with_token(
        std::env::var("ROM_EXTRAS_NATS_TOKEN").expect("required fixture token"),
    )
    .connection_timeout(Duration::from_secs(2))
    .connect(url)
    .await
    .unwrap_or_else(|_| panic!("fixture reconnect failed"));
    let context = jetstream::new(client);
    let mut stream = context.get_stream(&name).await.unwrap();
    assert_eq!(stream.info().await.unwrap().state.messages, 1);
    assert_eq!(
        stream.get_raw_message(1).await.unwrap().payload.as_ref(),
        b"false"
    );
    let adapter = JetStreamDelivery::bind(
        context.clone(),
        &name,
        &subject,
        1,
        Duration::from_secs(2),
        PayloadLimit::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        adapter
            .deliver(Delivery {
                id: "work-17".into(),
                attempt: 3,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    assert_eq!(stream.info().await.unwrap().state.messages, 1);
    let consumer: PullConsumer = stream.get_consumer("rom-extras-reader").await.unwrap();
    let mut messages = consumer
        .fetch()
        .max_messages(1)
        .expires(Duration::from_secs(2))
        .messages()
        .await
        .unwrap();
    let redelivery = tokio::time::timeout(Duration::from_secs(3), messages.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(redelivery.payload.as_ref(), b"false");
    assert_eq!(redelivery.info().unwrap().stream_sequence, 1);
    assert!(redelivery.info().unwrap().delivered >= 2);
    redelivery.double_ack().await.unwrap();
    drop(messages);
    let mut consumer: PullConsumer = stream.get_consumer("rom-extras-reader").await.unwrap();
    assert_eq!(consumer.info().await.unwrap().num_ack_pending, 0);
    let too_small = JetStreamDelivery::bind(
        context.clone(),
        &name,
        &subject,
        1,
        Duration::from_secs(2),
        PayloadLimit::new(4).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        too_small
            .deliver(Delivery {
                id: "refused-size".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Permanent
    );
    assert_eq!(stream.info().await.unwrap().state.messages, 1);
    let delayed = JetStreamDelivery::bind(
        context.clone(),
        &name,
        &subject,
        1,
        Duration::from_millis(200),
        PayloadLimit::default(),
    )
    .await
    .unwrap();
    let paused = PausedBroker::new();
    let (lost, overloaded) = tokio::join!(
        delayed.deliver(Delivery {
            id: "lost-ack-17".into(),
            attempt: 1,
            payload: false
        }),
        delayed.deliver(Delivery {
            id: "refused-overload".into(),
            attempt: 1,
            payload: false
        }),
    );
    assert_eq!(lost, DeliveryOutcome::Unknown);
    assert_eq!(overloaded, DeliveryOutcome::Retryable);
    drop(paused);
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if stream.info().await.unwrap().state.messages == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        stream.get_raw_message(2).await.unwrap().payload.as_ref(),
        b"false"
    );
    assert_eq!(
        adapter
            .deliver(Delivery {
                id: "lost-ack-17".into(),
                attempt: 2,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    assert_eq!(stream.info().await.unwrap().state.messages, 2);
    for invalid in [
        "",
        "events.*",
        "events.>",
        "events..notice",
        "$SYS.notice",
        "event\nnotice",
    ] {
        assert!(matches!(
            JetStreamDelivery::bind(
                context.clone(),
                &name,
                invalid,
                1,
                Duration::from_secs(2),
                PayloadLimit::default()
            )
            .await,
            Err(rom_nats::NatsDeliveryError::InvalidConfiguration)
        ));
    }
    assert!(matches!(
        JetStreamDelivery::bind(
            context.clone(),
            &name,
            &subject,
            0,
            Duration::from_secs(2),
            PayloadLimit::default()
        )
        .await,
        Err(rom_nats::NatsDeliveryError::InvalidConfiguration)
    ));
    assert!(matches!(
        JetStreamDelivery::bind(
            context.clone(),
            &name,
            &subject,
            1,
            Duration::ZERO,
            PayloadLimit::default()
        )
        .await,
        Err(rom_nats::NatsDeliveryError::InvalidConfiguration)
    ));
    let memory_name = format!("MEM_{name}");
    context
        .create_stream(Config {
            name: memory_name.clone(),
            subjects: vec![format!("mem.{name}")],
            storage: StorageType::Memory,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        JetStreamDelivery::bind(
            context,
            &memory_name,
            &format!("mem.{name}"),
            1,
            Duration::from_secs(2),
            PayloadLimit::default()
        )
        .await
        .is_err()
    );
}

// Always restore only the labelled fixture, including unwinding test failures.
struct PausedBroker;
impl PausedBroker {
    fn new() -> Self {
        assert!(
            std::process::Command::new("docker")
                .args(["pause", "rom-extras-nats-20261008"])
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
        let restored = std::process::Command::new("docker")
            .args(["unpause", "rom-extras-nats-20261008"])
            .stdout(std::process::Stdio::null())
            .status();
        assert!(
            restored.is_ok_and(|status| status.success()),
            "fixture unpause failed"
        );
    }
}
