//! Actual HTTPS receiver conformance; signatures verified independently in Node.
#![cfg(feature = "loopback-fixture")]
use rom::{Delivery, DeliveryOutcome};
use rom_delivery_core::{PayloadLimit, WebhookSigner};
use rom_webhook::{Destination, TransportLimits, Webhook};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Receiver {
    child: Child,
    port: u16,
    certificate: Vec<u8>,
}
impl Receiver {
    fn start() -> Self {
        let directory =
            std::env::temp_dir().join(format!("rom-extras-https-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let certificate = directory.join("certificate.pem");
        let key = directory.join("key.pem");
        let ca = directory.join("root.pem");
        let ca_key = directory.join("root-key.pem");
        let csr = directory.join("server.csr");
        let extensions = directory.join("server.ext");
        std::fs::write(&extensions, "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:receiver.example\n").unwrap();
        let result = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=ROM Extras fixture root",
                "-addext",
                "basicConstraints=critical,CA:TRUE",
                "-keyout",
            ])
            .arg(&ca_key)
            .arg("-out")
            .arg(&ca)
            .output()
            .unwrap();
        assert!(result.status.success(), "fixture root generation failed");
        let result = Command::new("openssl")
            .args([
                "req",
                "-new",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-subj",
                "/CN=receiver.example",
                "-keyout",
            ])
            .arg(&key)
            .arg("-out")
            .arg(&csr)
            .output()
            .unwrap();
        assert!(result.status.success(), "fixture CSR generation failed");
        let result = Command::new("openssl")
            .args(["x509", "-req", "-days", "1", "-in"])
            .arg(&csr)
            .arg("-CA")
            .arg(&ca)
            .arg("-CAkey")
            .arg(&ca_key)
            .arg("-CAcreateserial")
            .arg("-extfile")
            .arg(&extensions)
            .arg("-out")
            .arg(&certificate)
            .output()
            .unwrap();
        assert!(result.status.success(), "fixture leaf signing failed");
        let mut child = Command::new("node")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/receiver.cjs"))
            .arg(&key)
            .arg(&certificate)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        Self {
            child,
            port: line.trim().parse().unwrap(),
            certificate: std::fs::read(ca).unwrap(),
        }
    }
    fn client(&self, path: &str, limits: TransportLimits) -> Webhook {
        let endpoint = Destination::loopback_fixture(
            &format!("https://receiver.example:{}{path}", self.port),
            &["127.0.0.1".parse().unwrap()],
        )
        .unwrap();
        Webhook::with_fixture_root(
            endpoint,
            WebhookSigner::new([7; 32]),
            PayloadLimit::default(),
            limits,
            &self.certificate,
        )
        .unwrap()
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
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
    let stats_client = reqwest::Client::builder()
        .no_proxy()
        .https_only(true)
        .tls_certs_only([reqwest::Certificate::from_pem(&receiver.certificate).unwrap()])
        .resolve(
            "receiver.example",
            format!("127.0.0.1:{}", receiver.port).parse().unwrap(),
        )
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let stats = stats_client
        .get(format!("https://receiver.example:{}/stats", receiver.port))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
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
