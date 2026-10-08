//! Shared bounded HTTPS machinery; callers retain admission through decoding.
use crate::Limits;
use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderValue},
};
use rom_secrets::{Error, SecretBytes};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::{Semaphore, SemaphorePermit};
use url::Url;
use zeroize::Zeroizing;
pub(crate) struct Transport {
    client: Client,
    origin: Url,
    admission: Semaphore,
    limits: Limits,
}
impl Transport {
    pub(crate) fn new(
        endpoint: &str,
        token: SecretBytes,
        ca: Option<Vec<u8>>,
        limits: Limits,
    ) -> Result<Self, Error> {
        if !(1..=64).contains(&limits.max_in_flight)
            || !(1..=32768).contains(&limits.max_response_bytes)
            || !(Duration::from_millis(1)..=Duration::from_secs(60)).contains(&limits.deadline)
            || token.expose().is_empty()
        {
            return Err(Error::Invalid);
        }
        if endpoint.len() > 2048 {
            return Err(Error::Limit);
        }
        let origin = Url::parse(endpoint).map_err(|_| Error::Invalid)?;
        if origin.scheme() != "https"
            || origin.host().is_none()
            || origin.path() != "/"
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            return Err(Error::Invalid);
        }
        let mut token_header =
            HeaderValue::from_bytes(token.expose()).map_err(|_| Error::Invalid)?;
        token_header.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert("x-vault-token", token_header);
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        let mut builder = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .default_headers(headers)
            .connect_timeout(limits.deadline.min(Duration::from_secs(3)))
            .timeout(limits.deadline)
            .http1_only();
        if let Some(ca) = ca {
            if ca.len() > 32768 {
                return Err(Error::Limit);
            }
            builder = builder.add_root_certificate(
                reqwest::Certificate::from_pem(&ca).map_err(|_| Error::Invalid)?,
            );
        }
        Ok(Self {
            client: builder.build().map_err(|_| Error::Invalid)?,
            origin,
            admission: Semaphore::new(limits.max_in_flight),
            limits,
        })
    }
    pub(crate) fn admit(&self) -> Result<SemaphorePermit<'_>, Error> {
        self.admission.try_acquire().map_err(|_| Error::Busy)
    }
    pub(crate) fn url(&self, segments: &[&str]) -> Result<Url, Error> {
        let mut url = self.origin.clone();
        let mut path = url.path_segments_mut().map_err(|_| Error::Invalid)?;
        path.clear().push("v1");
        for segment in segments {
            path.push(segment);
        }
        drop(path);
        Ok(url)
    }
    pub(crate) async fn request(
        &self,
        method: Method,
        url: Url,
        body: Option<Vec<u8>>,
        rejection: Error,
    ) -> Result<Value, Error> {
        if body.as_ref().is_some_and(|body| body.len() > 32768) {
            return Err(Error::Limit);
        }
        let operation = async {
            let mut request = self.client.request(method, url);
            if let Some(body) = body {
                request = request.body(body);
            }
            let mut response = request.send().await.map_err(network_error)?;
            let status = response.status();
            let mut bytes = Zeroizing::new(Vec::with_capacity(self.limits.max_response_bytes));
            while let Some(chunk) = response.chunk().await.map_err(network_error)? {
                if chunk.len() > self.limits.max_response_bytes - bytes.len() {
                    return Err(Error::Limit);
                }
                bytes.extend_from_slice(&chunk);
            }
            if !status.is_success() {
                return Err(match status.as_u16() {
                    400 => rejection,
                    401 | 403 => Error::Denied,
                    404 => Error::NotFound,
                    429 => Error::Busy,
                    500..=599 => Error::Unavailable,
                    _ => Error::Protocol,
                });
            }
            serde_json::from_slice(&bytes).map_err(|_| Error::Protocol)
        };
        tokio::time::timeout(self.limits.deadline, operation)
            .await
            .map_err(|_| Error::Timeout)?
    }
}
fn network_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Unavailable
    }
}
