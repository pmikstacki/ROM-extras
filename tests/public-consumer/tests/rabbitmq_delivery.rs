//! RabbitMQ adapter consumed independently of workspace feature unification.
use lapin::{
    Connection, ConnectionProperties,
    options::*,
    types::{AMQPValue, FieldTable},
};
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_rabbitmq::RabbitMqDelivery;
use std::time::Duration;
#[tokio::test]
async fn external_consumer_receives_confirmed_persistent_false_payload() {
    let uri = std::env::var("ROM_EXTRAS_RABBITMQ_URI").expect("required RabbitMQ fixture");
    let connection = tokio::time::timeout(
        Duration::from_secs(5),
        Connection::connect(&uri, ConnectionProperties::default()),
    )
    .await
    .expect("connect deadline")
    .unwrap_or_else(|_| panic!("RabbitMQ fixture connection failed"));
    let channel = connection.create_channel().await.unwrap();
    let queue = format!(
        "CONSUMER_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut args = FieldTable::default();
    args.insert(
        "x-queue-type".into(),
        AMQPValue::LongString("quorum".into()),
    );
    channel
        .queue_declare(queue.clone().into(), QueueDeclareOptions::durable(), args)
        .await
        .unwrap();
    let adapter = RabbitMqDelivery::bind(
        &connection,
        &queue,
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
    let received = channel
        .basic_get(queue.into(), BasicGetOptions::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(received.data, b"false");
    assert_eq!(
        received.properties.message_id().as_ref().unwrap().as_str(),
        "external-consumer-17"
    );
    assert_eq!(received.properties.delivery_mode(), &Some(2));
    received.ack(BasicAckOptions::default()).await.unwrap();
}
