//! Actual verified-mTLS acknowledgement-loss relay owner.
use rom_opensearch::{OpenSearch, TlsConfig};
use rom_projection_core::DocumentMapping;
use std::{fs, time::Duration};
pub(crate) struct LossProxy {
    process: Option<fixture_process::FixtureProcess>,
    pub(crate) endpoint: String,
    pub(crate) control: Option<std::path::PathBuf>,
}
impl LossProxy {
    pub(crate) fn start() -> Self {
        Self::configured(None)
    }
    pub(crate) fn configured(control: Option<std::path::PathBuf>) -> Self {
        use std::process::Command;
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/support/response_loss_proxy.mjs");
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
            endpoint: format!("https://127.0.0.1:{port}"),
            control,
        }
    }
}
impl Drop for LossProxy {
    fn drop(&mut self) {
        drop(self.process.take());

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
    let root = "/root/ROM-extras/.superpowers/opensearch-fixture";
    let tls = TlsConfig::new(
        &proxy.endpoint,
        fs::read(format!("{root}/tls/ca.pem")).unwrap(),
        [
            fs::read(format!("{root}/tls/projection-writer.pem")).unwrap(),
            fs::read(format!("{root}/client-private/projection-writer.key")).unwrap(),
        ]
        .concat(),
        Duration::from_secs(5),
    )
    .unwrap();
    OpenSearch::new(
        tls,
        mapping.profile().clone(),
        physical,
        vec!["title".into()],
    )
    .unwrap()
}

#[path = "fixture_process.rs"]
mod fixture_process;
