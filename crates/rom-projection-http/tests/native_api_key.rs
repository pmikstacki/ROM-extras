//! Actual verified Qdrant API-key transport, separate from vector or adapter acceptance.
#![cfg(feature = "service-fixture")]
use reqwest::Method;
use rom_projection_core::TargetFailure;
use rom_projection_http::{Http, TlsConfig};
use std::{fs, time::Duration};
fn client(valid_key: bool, wrong_ca: bool) -> Http {
    let root = "/root/ROM-extras/.superpowers/qdrant-fixture";
    let credentials: serde_json::Value =
        serde_json::from_slice(&fs::read(format!("{root}/credentials.json")).unwrap()).unwrap();
    let key = if valid_key {
        credentials["read"].as_str().unwrap().as_bytes().to_vec()
    } else {
        b"unrecognized-fixture-key".to_vec()
    };
    let ca = fs::read(if wrong_ca {
        "/root/ROM-extras/.superpowers/opensearch-fixture/tls/projection-writer.pem".to_owned()
    } else {
        format!("{root}/tls/ca.pem")
    })
    .unwrap();
    Http::new(
        TlsConfig::api_key("https://127.0.0.1:55461", ca, key, Duration::from_secs(5)).unwrap(),
    )
    .unwrap()
}
#[tokio::test(flavor = "current_thread")]
async fn actual_api_key_header_is_required_and_tls_verification_cannot_be_bypassed() {
    let (status, body) = client(true, false)
        .request(Method::GET, &["collections"], &[], None, false)
        .await
        .unwrap();
    assert_eq!(status, 200);
    assert_eq!(body["status"], "ok");
    assert!(matches!(
        client(false, false)
            .request(Method::GET, &["collections"], &[], None, false)
            .await,
        Err(TargetFailure::Rejected)
    ));
    assert!(matches!(
        client(true, true)
            .request(Method::GET, &["collections"], &[], None, false)
            .await,
        Err(TargetFailure::Unknown)
    ));
}
