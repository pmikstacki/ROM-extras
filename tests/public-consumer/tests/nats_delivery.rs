//! Independent public consumer, without workspace feature unification.
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_nats::JetStreamDelivery;
use std::time::Duration;

#[tokio::test]
async fn public_adapter_gets_real_jetstream_acknowledgement() {
    let url = std::env::var("ROM_EXTRAS_NATS_URL").expect("required NATS fixture");
    let token = std::env::var("ROM_EXTRAS_NATS_TOKEN").expect("required NATS fixture token");
    let client = async_nats::ConnectOptions::with_token(token)
        .connection_timeout(Duration::from_secs(2))
        .connect(url)
        .await
        .unwrap_or_else(|_| panic!("NATS fixture connection failed"));
    let context = async_nats::jetstream::new(client);
    let name = format!(
        "CONSUMER_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let subject = format!("consumer.{name}");
    let mut stream = context
        .create_stream(async_nats::jetstream::stream::Config {
            name: name.clone(),
            subjects: vec![subject.clone()],
            storage: async_nats::jetstream::stream::StorageType::File,
            max_bytes: 1_048_576,
            max_message_size: 65_536,
            ..Default::default()
        })
        .await
        .unwrap();
    let adapter = JetStreamDelivery::bind(
        context,
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
                id: "external-consumer-17".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    assert_eq!(stream.info().await.unwrap().state.messages, 1);
    assert_eq!(
        stream.get_raw_message(1).await.unwrap().payload.as_ref(),
        b"false"
    );
}
