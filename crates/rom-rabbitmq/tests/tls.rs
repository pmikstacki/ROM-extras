//! Required native TLS peer verification and authenticated confirmed publication.
#[path = "common/fixture.rs"]
mod fixture;
use lapin::{
    Connection, ConnectionProperties,
    options::*,
    tcp::OwnedTLSConfig,
    types::{AMQPValue, FieldTable},
};
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_rabbitmq::RabbitMqDelivery;
use std::time::Duration;
async fn connect(uri: &str, ca: String) -> lapin::Result<Connection> {
    tokio::time::timeout(
        Duration::from_secs(4),
        Connection::connect_with_config(
            uri,
            ConnectionProperties::default(),
            OwnedTLSConfig {
                cert_chain: Some(ca),
                identity: None,
            },
            lapin::runtime::default_runtime().unwrap(),
        ),
    )
    .await
    .expect("bounded TLS handshake")
}
#[tokio::test]
async fn tls_verifies_ca_hostname_password_and_confirms_persistent_payload() {
    let uri = std::env::var("ROM_EXTRAS_RABBITMQ_TLS_URI").expect("required TLS fixture URI");
    assert!(
        uri.starts_with("amqps://rom_extras:") && uri.ends_with("@127.0.0.1:55447/rom_extras"),
        "dedicated TLS fixture required"
    );
    let ca = std::fs::read_to_string(
        std::env::var("ROM_EXTRAS_RABBITMQ_TLS_CA").expect("required CA path"),
    )
    .unwrap();
    let wrong = std::fs::read_to_string(
        std::env::var("ROM_EXTRAS_RABBITMQ_TLS_WRONG_CA").expect("required unrelated CA path"),
    )
    .unwrap();
    let mismatch = uri.replace("@127.0.0.1:", "@localhost:");
    let wrong_password = "amqps://rom_extras:synthetic-invalid-password@127.0.0.1:55447/rom_extras";
    let plaintext = "amqps://rom_extras:synthetic-invalid-password@127.0.0.1:55445/rom_extras";
    for (label, destination, root, cause) in [
        ("untrusted CA", uri.as_str(), wrong, "unknownissuer"),
        ("hostname", mismatch.as_str(), ca.clone(), "notvalidforname"),
        ("password", wrong_password, ca.clone(), "access_refused"),
        (
            "TLS to plaintext listener",
            plaintext,
            ca.clone(),
            "invalidcontenttype",
        ),
    ] {
        let error = match connect(destination, root).await {
            Ok(_) => panic!("negative TLS case unexpectedly connected: {label}"),
            Err(error) => error,
        };
        assert!(
            format!("{error:?}").to_ascii_lowercase().contains(cause),
            "negative connection failed for wrong cause: {label}"
        );
    }
    let connection = connect(&uri, ca.clone())
        .await
        .unwrap_or_else(|_| panic!("verified TLS fixture connection failed"));
    let channel = connection.create_channel().await.unwrap();
    let queue = format!(
        "TLS_{}_{}",
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
                id: "tls-confirmed-17".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    fixture::restart("rom-extras-rabbitmq-tls-20261008", "rabbitmq-tls").await;
    let connection = connect(&uri, ca)
        .await
        .unwrap_or_else(|_| panic!("verified TLS host reconnection failed"));
    let channel = connection.create_channel().await.unwrap();
    let info = channel
        .queue_declare(
            queue.clone().into(),
            QueueDeclareOptions {
                passive: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();
    assert_eq!(info.message_count(), 1);
    let rebound = RabbitMqDelivery::bind(
        &connection,
        &queue,
        Duration::from_secs(2),
        PayloadLimit::default(),
    )
    .await
    .unwrap();
    for (id, publish) in [("tls-confirmed-17", false), ("tls-confirmed-18", true)] {
        if publish {
            assert_eq!(
                rebound
                    .deliver(Delivery {
                        id: id.into(),
                        attempt: 1,
                        payload: false
                    })
                    .await,
                DeliveryOutcome::Accepted
            );
        }
        let received = channel
            .basic_get(queue.clone().into(), BasicGetOptions::default())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(received.data, b"false");
        assert_eq!(
            received.properties.message_id().as_ref().unwrap().as_str(),
            id
        );
        assert_eq!(received.properties.delivery_mode(), &Some(2));
        assert_eq!(
            received
                .properties
                .content_type()
                .as_ref()
                .unwrap()
                .as_str(),
            "application/json"
        );
        received.ack(BasicAckOptions::default()).await.unwrap();
    }
}
