//! Explicit fixture-only HTTP policy and private host credentials.
use std::{collections::HashMap, time::Duration};
pub fn client(case: &str) -> reqwest::blocking::Client {
    let mut builder = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(2))
        .connect_timeout(Duration::from_secs(1));
    if case != "untrusted_ca" {
        let ca = std::fs::read(std::env::var("ROM_EXTRAS_OTEL_CA").unwrap()).unwrap();
        builder = builder.add_root_certificate(reqwest::Certificate::from_pem(&ca).unwrap());
    }
    if case == "wrong_identity" {
        builder = builder.resolve("wrong.fixture.test", "10.246.72.2:4318".parse().unwrap());
    }
    if let Ok(address) = std::env::var("ROM_EXTRAS_OTEL_CONNECT_ADDRESS") {
        builder = builder.resolve("collector.fixture.test", address.parse().unwrap());
    }
    builder.build().unwrap()
}
pub fn headers(case: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    if case != "missing_auth" {
        let authorization = if case == "wrong_auth" {
            "Basic aW52YWxpZDppbnZhbGlk".into()
        } else {
            std::env::var("ROM_EXTRAS_OTEL_AUTHORIZATION").unwrap()
        };
        headers.insert("Authorization".into(), authorization);
    }
    headers
}
