use rom_delivery_core::{PayloadLimit, WebhookSigner};
use rom_webhook::{Destination, TransportLimits, Webhook};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};

pub struct Receiver {
    child: Child,
    directory: std::path::PathBuf,
    pub port: u16,
    pub certificate: Vec<u8>,
}
impl Receiver {
    pub fn start() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "rom-extras-https-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
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
        let (child, port) = Self::spawn(&directory);
        Self {
            child,
            directory,
            port,
            certificate: std::fs::read(ca).unwrap(),
        }
    }
    fn spawn(directory: &std::path::Path) -> (Child, u16) {
        let mut child = Command::new("node")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/receiver.cjs"))
            .arg(directory.join("key.pem"))
            .arg(directory.join("certificate.pem"))
            .arg(directory.join("receiver.sqlite"))
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        (child, line.trim().parse().unwrap())
    }
    // Shared fixture method is unused by the separate transport-only target.
    #[allow(dead_code)]
    pub fn restart(&mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
        let (child, port) = Self::spawn(&self.directory);
        self.child = child;
        self.port = port;
    }
    pub async fn stats(&self) -> String {
        let client = reqwest::Client::builder()
            .no_proxy()
            .https_only(true)
            .tls_certs_only([reqwest::Certificate::from_pem(&self.certificate).unwrap()])
            .resolve(
                "receiver.example",
                format!("127.0.0.1:{}", self.port).parse().unwrap(),
            )
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        client
            .get(format!("https://receiver.example:{}/stats", self.port))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    }
    pub fn client(&self, path: &str, limits: TransportLimits) -> Webhook {
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
