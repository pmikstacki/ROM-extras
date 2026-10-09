//! Owned verified relay with a bounded actual-response barrier.
use crate::fixture_process;
use std::{fs, path::PathBuf, process::Command, time::Duration};
pub(crate) struct SearchProxy {
    process: Option<fixture_process::FixtureProcess>,
    control: PathBuf,
    pub(crate) endpoint: String,
}
impl SearchProxy {
    pub(crate) fn start() -> Self {
        use std::os::unix::fs::DirBuilderExt;
        let control = std::env::temp_dir().join(format!(
            "rom-os-search-hold-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&control).unwrap();
        let script = control.join("relay.mjs");
        fs::write(&script, include_str!("search_response_proxy.mjs")).unwrap();
        let mut command = Command::new("node");
        command
            .arg(script)
            .arg("/root/ROM-extras/.superpowers/opensearch-fixture")
            .arg(&control);
        let (process, port) =
            fixture_process::FixtureProcess::start(&mut command, Duration::from_secs(5)).unwrap();
        Self {
            process: Some(process),
            control,
            endpoint: format!("https://127.0.0.1:{port}"),
        }
    }
    pub(crate) async fn held(&self) {
        let wait = async {
            loop {
                if let Ok(raw) = fs::read(self.control.join("held")) {
                    let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
                    assert_eq!(value["http"], 200);
                    assert_eq!(value["timed_out"], false);
                    assert_eq!(value["candidates"], 4);
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        };
        tokio::time::timeout(Duration::from_secs(4), wait)
            .await
            .unwrap();
    }
    pub(crate) fn request_count(&self) -> usize {
        fs::read_to_string(self.control.join("requests"))
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    }
    pub(crate) fn release(&self) {
        fs::write(self.control.join("release"), b"release actual response").unwrap();
    }
}
impl Drop for SearchProxy {
    fn drop(&mut self) {
        drop(self.process.take());
        fs::remove_dir_all(&self.control).unwrap();
    }
}
