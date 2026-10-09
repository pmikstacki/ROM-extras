//! Bounded fixed-origin HTTP without redirects, proxies or implicit retries.
use crate::TlsConfig;
use reqwest::{Client, Method};
use rom_projection_core::{Error, Result, TargetFailure};
use serde_json::Value;
use std::time::Duration;
use url::Url;
pub(crate) const WIRE_LIMIT: usize = 1048576;
pub(crate) struct Transport {
    client: Client,
    origin: Url,
    pub(crate) deadline: Duration,
}
impl Transport {
    pub(crate) fn new(config: TlsConfig) -> Result<Self> {
        let client = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .http1_only()
            .connect_timeout(config.deadline.min(Duration::from_secs(3)))
            .timeout(config.deadline)
            .add_root_certificate(
                reqwest::Certificate::from_pem(&config.ca).map_err(|_| Error::Invalid)?,
            )
            .identity(reqwest::Identity::from_pem(&config.identity).map_err(|_| Error::Invalid)?)
            .build()
            .map_err(|_| Error::Invalid)?;
        Ok(Self {
            client,
            origin: config.origin,
            deadline: config.deadline,
        })
    }
    pub(crate) async fn request(
        &self,
        method: Method,
        segments: &[&str],
        query: &[(&str, &str)],
        body: Option<Vec<u8>>,
        ndjson: bool,
    ) -> std::result::Result<(u16, Value), TargetFailure> {
        if body.as_ref().is_some_and(|b| b.len() > WIRE_LIMIT) {
            return Err(TargetFailure::Rejected);
        }
        let operation = async {
            let mut url = self.origin.clone();
            {
                let mut path = url
                    .path_segments_mut()
                    .map_err(|_| TargetFailure::Rejected)?;
                path.clear();
                for s in segments {
                    path.push(s);
                }
            }
            url.query_pairs_mut().extend_pairs(query.iter().copied());
            let mut request = self.client.request(method, url).header(
                "content-type",
                if ndjson {
                    "application/x-ndjson"
                } else {
                    "application/json"
                },
            );
            if let Some(body) = body {
                request = request.body(body);
            }
            let mut response = request.send().await.map_err(|_| TargetFailure::Unknown)?;
            let status = response.status().as_u16();
            if !(200..300).contains(&status) {
                return Err(if (400..500).contains(&status) && status != 429 {
                    TargetFailure::Rejected
                } else {
                    TargetFailure::Unknown
                });
            }
            if response
                .content_length()
                .is_some_and(|n| n > WIRE_LIMIT as u64)
            {
                return Err(TargetFailure::Unknown);
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| TargetFailure::Unknown)? {
                if chunk.len() > WIRE_LIMIT - bytes.len() {
                    return Err(TargetFailure::Unknown);
                }
                bytes.extend_from_slice(&chunk);
            }
            let value = serde_json::from_slice(&bytes).map_err(|_| TargetFailure::Unknown)?;
            Ok((status, value))
        };
        tokio::time::timeout(self.deadline, operation)
            .await
            .map_err(|_| TargetFailure::Unknown)?
    }
}
