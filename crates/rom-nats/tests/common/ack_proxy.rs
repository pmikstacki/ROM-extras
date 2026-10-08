use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
};
pub struct AckProxy {
    child: Child,
    pub url: String,
    evidence: PathBuf,
}
impl AckProxy {
    pub fn start(stream: &str) -> Self {
        let evidence =
            std::env::temp_dir().join(format!("rom-nats-ack-{}-{stream}.json", std::process::id()));
        let mut child = Command::new("node")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/common/ack-proxy.cjs"
            ))
            .arg(stream)
            .arg(&evidence)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        // Construct the guard before waiting: failed readiness also terminates the child.
        let mut proxy = Self {
            child,
            url: String::new(),
            evidence,
        };
        let (ready, received) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            let _ = ready.send(result);
        });
        let line = received
            .recv_timeout(std::time::Duration::from_secs(3))
            .expect("bounded proxy readiness")
            .expect("proxy readiness read");
        let port: u16 = line.trim().parse().expect("proxy readiness port");
        assert_ne!(port, 0);
        proxy.url = format!("nats://127.0.0.1:{port}");
        proxy
    }
    pub fn dropped(&self, stream: &str) -> bool {
        std::fs::read_to_string(&self.evidence).is_ok_and(|value| value == format!("{stream}:1\n"))
    }
}
impl Drop for AckProxy {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
