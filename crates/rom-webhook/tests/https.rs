//! Actual HTTPS receiver conformance; signatures verified independently in Node.
#![cfg(feature = "loopback-fixture")]
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::{PayloadLimit, WebhookSigner};
use rom_webhook::{Destination, TransportLimits, Webhook};
use std::time::Duration;
mod common;
use common::Receiver;
fn delivery(attempt: u32) -> Delivery<bool> {
    Delivery {
        id: "work-17".into(),
        attempt,
        payload: false,
    }
}

// Bad signature framing/body, hidden retries/redirects, or false success fail here.
#[tokio::test]
async fn delivers_signed_exact_bytes_and_preserves_uncertainty() {
    let receiver = Receiver::start();
    let limits = TransportLimits::new(1, Duration::from_secs(1), Duration::from_secs(2)).unwrap();
    for (path, expected) in [
        ("/accept", DeliveryOutcome::Accepted),
        ("/redirect", DeliveryOutcome::Permanent),
        ("/reject", DeliveryOutcome::Permanent),
        ("/server-error", DeliveryOutcome::Unknown),
        ("/disconnect", DeliveryOutcome::Unknown),
    ] {
        assert_eq!(
            receiver
                .client(path, limits)
                .deliver(delivery(1), 1_800_000_000)
                .await,
            expected,
            "{path}"
        );
    }
    let client = receiver.client("/deduplicate", limits);
    assert_eq!(
        client.deliver(delivery(1), 1_800_000_000).await,
        DeliveryOutcome::Accepted
    );
    assert_eq!(
        client.deliver(delivery(2), 1_800_000_001).await,
        DeliveryOutcome::Accepted
    );
    let client = receiver.client("/slow", limits);
    let (first, second) = tokio::join!(
        client.deliver(delivery(1), 1_800_000_000),
        client.deliver(delivery(2), 1_800_000_001)
    );
    assert_eq!(first, DeliveryOutcome::Accepted);
    assert_eq!(second, DeliveryOutcome::Retryable);
    let endpoint = Destination::loopback_fixture(
        &format!("https://receiver.example:{}/accept", receiver.port),
        &["127.0.0.1".parse().unwrap()],
    )
    .unwrap();
    let untrusted = Webhook::new(
        endpoint,
        WebhookSigner::new([7; 32]),
        PayloadLimit::default(),
        limits,
    )
    .unwrap();
    assert_eq!(
        untrusted.deliver(delivery(1), 1_800_000_000).await,
        DeliveryOutcome::Unknown
    );
    let endpoint = Destination::loopback_fixture(
        &format!("https://receiver.example:{}/accept", receiver.port),
        &["127.0.0.1".parse().unwrap()],
    )
    .unwrap();
    let too_small = Webhook::with_fixture_root(
        endpoint,
        WebhookSigner::new([7; 32]),
        PayloadLimit::new(4).unwrap(),
        limits,
        &receiver.certificate,
    )
    .unwrap();
    assert_eq!(
        too_small.deliver(delivery(1), 1_800_000_000).await,
        DeliveryOutcome::Permanent
    );
    let short =
        TransportLimits::new(1, Duration::from_millis(100), Duration::from_millis(200)).unwrap();
    assert_eq!(
        receiver
            .client("/timeout", short)
            .deliver(delivery(1), 1_800_000_000)
            .await,
        DeliveryOutcome::Unknown
    );
    let stats = receiver.stats().await;
    for line in [
        "/accept=1",
        "/redirect=1",
        "/reject=1",
        "/server-error=1",
        "/disconnect=1",
        "/slow=1",
        "/timeout=1",
        "/deduplicate=2",
        "deduplicated-effects=1",
    ] {
        assert!(
            stats.lines().any(|actual| actual == line),
            "missing {line}: {stats}"
        );
    }
    assert!(!stats.lines().any(|line| line.starts_with("/trap=")));
}
