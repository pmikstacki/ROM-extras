//! CA/hostname/auth verification and reconnection against the required TLS-first broker fixture.
#[path = "common/fixture_name.rs"]
mod fixture_name;
use async_nats::{
    ConnectOptions, Event,
    jetstream::{
        self,
        stream::{Config, StorageType},
    },
};
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::PayloadLimit;
use rom_nats::JetStreamDelivery;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
fn options(token: String, ca: PathBuf) -> ConnectOptions {
    ConnectOptions::with_token(token)
        .require_tls(true)
        .tls_first()
        .add_root_certificates(ca)
        .connection_timeout(Duration::from_secs(2))
        .request_timeout(Some(Duration::from_secs(2)))
        .max_reconnects(20)
        .client_capacity(64)
        .subscription_capacity(64)
        .ignore_discovered_servers()
        .reconnect_delay_callback(|attempt| {
            Duration::from_millis((100_u64.saturating_mul(attempt as u64 + 1)).min(1000))
        })
}
#[tokio::test]
async fn tls_first_verifies_identity_and_reconnects_same_client_after_restart() {
    let url = std::env::var("ROM_EXTRAS_NATS_TLS_URL").expect("required dedicated TLS fixture URL");
    let fixture = fixture_name::selected(true);
    fixture_name::assert_endpoint(&fixture, true, &url);
    let token = std::env::var("ROM_EXTRAS_NATS_TLS_TOKEN").expect("required TLS fixture token");
    let ca =
        PathBuf::from(std::env::var("ROM_EXTRAS_NATS_TLS_CA").expect("required TLS fixture CA"));
    let wrong_ca = PathBuf::from(
        std::env::var("ROM_EXTRAS_NATS_TLS_WRONG_CA").expect("required unrelated fixture CA"),
    );
    let mismatched_hostname = url.replace("127.0.0.1", "localhost");
    let plain_url = std::env::var("ROM_EXTRAS_NATS_URL").expect("required plain fixture URL");
    let plain_fixture = fixture_name::selected(false);
    fixture_name::assert_endpoint(&plain_fixture, false, &plain_url);
    for (label, destination, credentials, root) in [
        ("untrusted CA", url.as_str(), token.clone(), wrong_ca),
        (
            "hostname mismatch",
            mismatched_hostname.as_str(),
            token.clone(),
            ca.clone(),
        ),
        (
            "wrong token",
            url.as_str(),
            "synthetic-invalid-token".into(),
            ca.clone(),
        ),
        (
            "plaintext fallback",
            plain_url.as_str(),
            token.clone(),
            ca.clone(),
        ),
    ] {
        let result = tokio::time::timeout(
            Duration::from_secs(4),
            options(credentials, root).connect(destination),
        )
        .await
        .expect("finite negative connect");
        let error = result
            .err()
            .unwrap_or_else(|| panic!("negative profile accepted: {label}"));
        println!("negative profile {label}: {}", error.kind());
        let diagnostic = error.to_string();
        if label == "hostname mismatch" {
            println!("hostname-only TLS diagnostic: {diagnostic}");
        }
        match label {
            "untrusted CA" => assert!(
                diagnostic.contains("UnknownIssuer"),
                "CA rejection must identify untrusted issuer"
            ),
            "hostname mismatch" => assert!(
                diagnostic.contains("certificate not valid for name"),
                "hostname rejection must identify name mismatch"
            ),
            "wrong token" => assert_eq!(
                error.kind(),
                async_nats::ConnectErrorKind::AuthorizationViolation
            ),
            "plaintext fallback" => assert_eq!(error.kind(), async_nats::ConnectErrorKind::Io),
            _ => unreachable!(),
        }
    }
    let connected = Arc::new(AtomicUsize::new(0));
    let disconnected = Arc::new(AtomicUsize::new(0));
    let events_connected = connected.clone();
    let events_disconnected = disconnected.clone();
    let client = options(token, ca)
        .event_callback(move |event| {
            let c = events_connected.clone();
            let d = events_disconnected.clone();
            async move {
                match event {
                    Event::Connected => {
                        c.fetch_add(1, Ordering::SeqCst);
                    }
                    Event::Disconnected => {
                        d.fetch_add(1, Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
        })
        .connect(&url)
        .await
        .unwrap_or_else(|_| panic!("CA-verified TLS fixture connect failed"));
    let context = jetstream::new(client);
    let name = format!(
        "TLS_{}_{}",
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
            duplicate_window: Duration::from_secs(120),
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
                id: "tls-restart".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    assert!(
        tokio::task::spawn_blocking(move || std::process::Command::new("docker")
            .args(["restart", "--time", "10", &fixture])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success())
        .await
        .unwrap()
    );
    tokio::time::timeout(Duration::from_secs(10), async {
        while connected.load(Ordering::SeqCst) < 2 || disconnected.load(Ordering::SeqCst) < 1 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("same TLS client must reconnect after real broker restart");
    assert_eq!(stream.info().await.unwrap().state.messages, 1);
    assert_eq!(
        adapter
            .deliver(Delivery {
                id: "tls-restart".into(),
                attempt: 2,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    assert_eq!(stream.info().await.unwrap().state.messages, 1);
    assert_eq!(
        adapter
            .deliver(Delivery {
                id: "tls-after-reconnect".into(),
                attempt: 1,
                payload: false
            })
            .await,
        DeliveryOutcome::Accepted
    );
    assert_eq!(stream.info().await.unwrap().state.messages, 2);
    assert_eq!(
        stream.get_raw_message(2).await.unwrap().payload.as_ref(),
        b"false"
    );
}
