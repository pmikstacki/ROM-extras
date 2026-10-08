//! Real post-persistence acknowledgement loss, independently witnessed through the broker.
mod common;
use async_nats::jetstream::{
    self,
    stream::{Config, StorageType},
};
use common::AckProxy;
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_nats::JetStreamDelivery;
use std::time::Duration;

#[tokio::test]
async fn dropped_publish_ack_preserves_message_and_retry_identity() {
    assert_eq!(
        std::env::var("ROM_EXTRAS_NATS_URL").expect("required fixture URL"),
        "nats://127.0.0.1:55441"
    );
    let token = std::env::var("ROM_EXTRAS_NATS_TOKEN").expect("required fixture token");
    let direct = async_nats::ConnectOptions::with_token(token.clone())
        .connection_timeout(Duration::from_secs(2))
        .connect("nats://127.0.0.1:55441")
        .await
        .unwrap_or_else(|_| panic!("fixture connect failed"));
    let context = jetstream::new(direct);
    let name = format!(
        "ACK_{}_{}",
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
            duplicate_window: Duration::from_secs(120),
            max_bytes: 1_048_576,
            ..Default::default()
        })
        .await
        .unwrap();
    let proxy = AckProxy::start(&name);
    let client = async_nats::ConnectOptions::with_token(token)
        .ignore_discovered_servers()
        .connection_timeout(Duration::from_secs(2))
        .connect(&proxy.url)
        .await
        .unwrap_or_else(|_| panic!("proxy connect failed"));
    let adapter = JetStreamDelivery::bind(
        jetstream::new(client),
        &name,
        &subject,
        1,
        Duration::from_secs(2),
        PayloadLimit::default(),
    )
    .await
    .unwrap();
    let completed = std::sync::atomic::AtomicBool::new(false);
    let first = async {
        let outcome = adapter
            .deliver(Delivery {
                id: "committed-lost-ack".into(),
                attempt: 1,
                payload: false,
            })
            .await;
        completed.store(true, std::sync::atomic::Ordering::SeqCst);
        outcome
    };
    let witness = async {
        tokio::time::timeout(Duration::from_secs(1), async {
            while !proxy.dropped(&name) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("actual PubAck must be dropped before attempt deadline");
        assert_eq!(stream.info().await.unwrap().state.messages, 1);
        let raw = stream.get_raw_message(1).await.unwrap();
        assert_eq!(raw.payload.as_ref(), b"false");
        assert_eq!(
            raw.headers.get("Nats-Msg-Id").unwrap().as_str(),
            "committed-lost-ack"
        );
        assert!(
            !completed.load(std::sync::atomic::Ordering::SeqCst),
            "broker witness must complete while delivery remains pending"
        );
    };
    let (outcome, ()) = tokio::join!(first, witness);
    assert_eq!(outcome, DeliveryOutcome::Unknown);
    assert_eq!(
        adapter
            .deliver(Delivery {
                id: "committed-lost-ack".into(),
                attempt: 2,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    assert_eq!(stream.info().await.unwrap().state.messages, 1);
}
