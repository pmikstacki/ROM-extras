use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};
pub struct AckProxy {
    child: Child,
    pub port: u16,
    evidence: PathBuf,
}
impl AckProxy {
    pub fn start(topic: &str) -> Self {
        let evidence =
            std::env::temp_dir().join(format!("rom-kafka-ack-{}-{topic}.json", std::process::id()));
        let mut child = Command::new("node")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/common/ack-proxy.cjs"
            ))
            .arg(&evidence)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut proxy = Self {
            child,
            port: 0,
            evidence,
        };
        let (ready, received) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            let _ = ready.send(result);
        });
        proxy.port = received
            .recv_timeout(Duration::from_secs(3))
            .expect("bounded proxy readiness")
            .expect("proxy readiness read")
            .trim()
            .parse()
            .expect("proxy port");
        assert_ne!(proxy.port, 0);
        proxy
    }
    pub fn dropped(&self) -> bool {
        std::fs::read_to_string(&self.evidence).is_ok_and(|value| {
            value.starts_with("{\"api\":0,") && value.ends_with("\"partition\":0,\"offset\":\"0\"}")
        })
    }
}
impl Drop for AckProxy {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
