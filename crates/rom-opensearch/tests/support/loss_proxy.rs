//! Actual verified-mTLS acknowledgement-loss relay owner.
use rom_opensearch::OpenSearch;
use rom_projection_core::DocumentMapping;
use std::{fs, time::Duration};
pub(crate) struct LossProxy {
    process: Option<fixture_process::FixtureProcess>,
    script_directory: std::path::PathBuf,
    pub(crate) endpoint: String,
    pub(crate) control: Option<std::path::PathBuf>,
}
impl LossProxy {
    pub(crate) fn start() -> Self {
        Self::configured(None)
    }
    pub(crate) fn configured(control: Option<std::path::PathBuf>) -> Self {
        use std::os::unix::fs::DirBuilderExt;
        use std::process::Command;
        let script_directory = std::env::temp_dir().join(format!(
            "rom-os-loss-relay-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&script_directory)
            .unwrap();
        let script = script_directory.join("relay.mjs");
        fs::write(&script, include_str!("response_loss_proxy.mjs")).unwrap();
        let mut command = Command::new("node");
        command
            .arg(script)
            .arg("/root/ROM-extras/.superpowers/opensearch-fixture");
        if let Some(path) = &control {
            command.arg("delay").arg(path);
        }
        let (process, port) =
            fixture_process::FixtureProcess::start(&mut command, Duration::from_secs(5)).unwrap();
        Self {
            process: Some(process),
            script_directory,
            endpoint: format!("https://127.0.0.1:{port}"),
            control,
        }
    }
}
impl Drop for LossProxy {
    fn drop(&mut self) {
        drop(self.process.take());
        fs::remove_dir_all(&self.script_directory).unwrap();

        if let Some(path) = &self.control {
            fs::remove_dir_all(path).unwrap();
        }
    }
}
pub(crate) fn lossy_target(
    proxy: &LossProxy,
    mapping: &DocumentMapping,
    physical: &str,
) -> OpenSearch {
    crate::native_fixture::target_at(
        &proxy.endpoint,
        mapping.profile().clone(),
        physical,
        vec!["title".into()],
    )
}

use crate::fixture_process;
