//! Real request delay barrier, used only by delayed-write qualification.
use std::{fs, time::Duration};
impl super::loss_proxy::LossProxy {
    pub(crate) fn delayed() -> Self {
        let control = std::env::temp_dir().join(format!(
            "rom-os-delay-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(&control).unwrap();
        Self::configured(Some(control))
    }
}
impl super::loss_proxy::LossProxy {
    pub(crate) async fn marker(&self, name: &str) -> String {
        let path = self.control.as_ref().unwrap().join(name);
        let wait = async {
            loop {
                if let Ok(value) = fs::read_to_string(&path) {
                    return value;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        };
        tokio::time::timeout(Duration::from_secs(10), wait)
            .await
            .unwrap()
    }
}
