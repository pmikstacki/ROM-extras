//! Test-only administration; runtime adapter never receives the root token.
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use rom_secrets::Error;
use serde_json::{Value, json};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
pub(crate) fn unique() -> String {
    format!(
        "case_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}
pub(crate) struct Admin {
    client: Client,
}
impl Admin {
    pub(crate) async fn status(&self, path: &str) -> u16 {
        assert!(!path.contains('?') && !path.contains('#') && !path.contains(".."));
        self.client
            .get(format!("https://127.0.0.1:55459/v1/{path}"))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16()
    }
    pub(crate) async fn unseal_existing(&self) {
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            if let Ok(response) = self
                .client
                .get("https://127.0.0.1:55459/v1/sys/seal-status")
                .send()
                .await
                && response.status().is_success()
            {
                let status = read_response(response).await.unwrap();
                assert_eq!(status["initialized"], true);
                assert_eq!(status["sealed"], true);
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "fixture listener did not return"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let data: Value = serde_json::from_slice(
            &std::fs::read(std::env::var("ROM_EXTRAS_OPENBAO_ADMIN_FILE").unwrap()).unwrap(),
        )
        .unwrap();
        let response = self
            .post("sys/unseal", json!({"key":data["keys_base64"][0]}))
            .await
            .unwrap();
        assert_eq!(response["sealed"], false);
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            if let Ok(response) = self
                .client
                .get("https://127.0.0.1:55459/v1/sys/health")
                .send()
                .await
                && response.status().as_u16() == 200
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "fixture did not become active"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    pub(crate) fn new() -> Self {
        assert_eq!(
            std::env::var("ROM_EXTRAS_OPENBAO_ENDPOINT").unwrap(),
            "https://127.0.0.1:55459"
        );
        let data: Value = serde_json::from_slice(
            &std::fs::read(
                std::env::var("ROM_EXTRAS_OPENBAO_ADMIN_FILE")
                    .expect("protected admin fixture response required"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut token = HeaderValue::from_str(data["root_token"].as_str().unwrap()).unwrap();
        token.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert("x-vault-token", token);
        let ca = std::fs::read(std::env::var("ROM_EXTRAS_OPENBAO_CA").unwrap()).unwrap();
        let client = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .add_root_certificate(reqwest::Certificate::from_pem(&ca).unwrap())
            .build()
            .unwrap();
        Self { client }
    }
    pub(crate) async fn post(&self, path: &str, payload: Value) -> Result<Value, Error> {
        assert!(!path.contains('?') && !path.contains('#') && !path.contains(".."));
        let response = self
            .client
            .post(format!("https://127.0.0.1:55459/v1/{path}"))
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&payload).map_err(|_| Error::Invalid)?)
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        read_response(response).await
    }
    pub(crate) async fn token(&self, name: &str, path: &str, key: &str) -> String {
        let policy = format!(
            "path \"secret/data/{path}\" {{ capabilities = [\"read\"] }}\npath \"transit/encrypt/{key}\" {{ capabilities = [\"update\"] }}\npath \"transit/decrypt/{key}\" {{ capabilities = [\"update\"] }}\n"
        );
        self.post(
            &format!("sys/policies/acl/{name}"),
            json!({"policy":policy}),
        )
        .await
        .unwrap();
        let value = self
            .post(
                "auth/token/create",
                json!({"policies":[name],"no_default_policy":true,"ttl":"1h","renewable":false}),
            )
            .await
            .unwrap();
        value["auth"]["client_token"].as_str().unwrap().to_owned()
    }
}
async fn read_response(mut response: reqwest::Response) -> Result<Value, Error> {
    let status = response.status();
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
        if body.len() + chunk.len() > 32768 {
            return Err(Error::Limit);
        }
        body.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        eprintln!(
            "administrative fixture response status: {}",
            status.as_u16()
        );
        return Err(Error::Protocol);
    }
    if body.is_empty() {
        Ok(Value::Null)
    } else {
        serde_json::from_slice(&body).map_err(|_| Error::Protocol)
    }
}
