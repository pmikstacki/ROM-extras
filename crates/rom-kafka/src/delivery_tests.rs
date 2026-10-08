use super::*;
#[tokio::test]
async fn real_native_queue_full_refuses_before_enqueue_without_retiring() {
    // A bound listener cannot complete Kafka metadata or acknowledgement.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut config = ClientConfig::new();
    config.set(
        "bootstrap.servers",
        listener.local_addr().unwrap().to_string(),
    );
    let adapter = KafkaDelivery::new(
        config,
        "native-queue",
        Duration::from_secs(3),
        PayloadLimit::default(),
    )
    .unwrap();
    let queued = adapter
        .producer
        .send_result(
            FutureRecord::to("native-queue")
                .key("held-17")
                .payload("false"),
        )
        .expect("first native queue slot");
    assert_eq!(
        adapter
            .deliver(Delivery {
                id: "refused-18".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Retryable
    );
    assert!(!adapter.retired.load(Ordering::Acquire));
    drop(queued);
}
