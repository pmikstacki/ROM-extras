//! Verified administrative fixture client; never used by production transport.
use std::{fs, time::Duration};
pub(super) fn client() -> reqwest::Client {
    let root = "/root/ROM-extras/.superpowers/opensearch-fixture";
    let identity = [
        fs::read(format!("{root}/tls/admin.pem")).unwrap(),
        fs::read(format!("{root}/client-private/admin.key")).unwrap(),
    ]
    .concat();
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .http1_only()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(3))
        .add_root_certificate(
            reqwest::Certificate::from_pem(&fs::read(format!("{root}/tls/ca.pem")).unwrap())
                .unwrap(),
        )
        .identity(reqwest::Identity::from_pem(&identity).unwrap())
        .build()
        .unwrap()
}
