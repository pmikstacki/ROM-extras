//! Actual successful ProduceResponse loss with an independent pre-completion broker witness.
#[path = "common/ack_proxy.rs"]
mod ack_proxy;
mod common;
use ack_proxy::AckProxy;
use common::fixture::{config, consumer, provision, read};
use rdkafka::{Message, message::Headers};
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_kafka::KafkaDelivery;
use std::time::Duration;
fn delivery(attempt: u32) -> Delivery<bool> {
    Delivery {
        id: "lost-ack-17".into(),
        attempt,
        payload: false,
    }
}
#[tokio::test]
async fn successful_broker_write_precedes_unknown_and_same_identity_retry() {
    let topic = format!(
        "ROMEXTRAS_ACK_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    provision(&topic);
    let proxy = AckProxy::start(&topic);
    let mut cfg = config();
    cfg.set("bootstrap.servers", format!("127.0.0.1:{}", proxy.port));
    let adapter = KafkaDelivery::new(
        cfg,
        &topic,
        Duration::from_secs(10),
        PayloadLimit::default(),
    )
    .unwrap();
    let reader = consumer(&topic);
    let mut pending = Box::pin(adapter.deliver(delivery(1)));
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            tokio::select! {
                outcome = &mut pending => panic!("delivery ended before broker witness: {outcome:?}"),
                () = tokio::time::sleep(Duration::from_millis(10)) => {}
            }
            if proxy.dropped() { break; }
        }
    }).await.expect("bounded actual ProduceResponse drop");
    let received = read(&reader);
    assert_eq!(received.offset(), 0);
    assert_eq!(received.key(), Some(b"lost-ack-17".as_slice()));
    assert_eq!(received.payload(), Some(b"false".as_slice()));
    assert_eq!(
        received.headers().unwrap().get(0).value,
        Some(b"application/json".as_slice())
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(1), &mut pending)
            .await
            .is_err(),
        "broker witness precedes outcome"
    );
    assert_eq!(pending.await, DeliveryOutcome::Unknown);
    assert_eq!(
        adapter.deliver(delivery(2)).await,
        DeliveryOutcome::Retryable
    );
    let rebound = KafkaDelivery::new(
        config(),
        &topic,
        Duration::from_secs(3),
        PayloadLimit::default(),
    )
    .unwrap();
    assert_eq!(
        rebound.deliver(delivery(2)).await,
        DeliveryOutcome::Accepted
    );
    let repeated = read(&reader);
    assert_eq!(repeated.offset(), 1);
    assert_eq!(repeated.key(), received.key());
    assert_eq!(repeated.payload(), received.payload());
    assert!(reader.poll(Duration::from_millis(200)).is_none());
}
