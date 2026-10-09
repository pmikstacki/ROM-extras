//! Map-specific bounded reads, separate from projection write outcomes.
use crate::Config;
use rom_map_core::{Error, RequestContext, Result};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, Semaphore},
    time::Instant,
};
/// Host-configured service reader; no implicit provider or browser activation.
pub struct Http {
    config: Config,
    client: reqwest::Client,
    admission: Arc<Semaphore>,
    next: Mutex<Option<Instant>>,
}
impl Http {
    /// Prepare a reader without opening a connection.
    pub fn new(config: Config) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .tls_backend_rustls()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .http1_only()
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(60))
            .user_agent(&config.agent);
        if let Some(ca) = &config.ca {
            let ca = reqwest::Certificate::from_pem(ca).map_err(|_| Error::InvalidQuery)?;
            builder = builder.tls_certs_only(vec![ca]);
        }
        let client = builder.build().map_err(|_| Error::Unavailable)?;
        let admission = Arc::new(Semaphore::new(config.concurrency));
        Ok(Self {
            config,
            client,
            admission,
            next: Mutex::new(None),
        })
    }
    /// Explicitly admit bounded native HTTP400 data for adapter-side normalization.
    /// This is not a browser response or a safe error message.
    pub async fn get_native(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
        context: &RequestContext,
    ) -> Result<crate::NativeResponse> {
        self.read(segments, query, context, true).await
    }
    /// Read one bounded response using the host's total operation context.
    pub async fn get(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
        context: &RequestContext,
    ) -> Result<Vec<u8>> {
        self.read(segments, query, context, false)
            .await
            .map(|response| response.body)
    }
    async fn read(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
        context: &RequestContext,
        admit_bad_request: bool,
    ) -> Result<crate::NativeResponse> {
        context
            .run(async {
                let _permit = self
                    .admission
                    .clone()
                    .try_acquire_owned()
                    .map_err(|_| Error::Unavailable)?;
                let url = super::request_url::prepare(&self.config, segments, query)?;
                {
                    let mut next = self.next.lock().await;
                    if let Some(next) = *next {
                        tokio::time::sleep_until(next).await;
                    }
                    *next = Some(Instant::now() + self.config.interval);
                }
                let response = self
                    .client
                    .get(url)
                    .header(reqwest::header::ACCEPT, "application/json")
                    .send()
                    .await
                    .map_err(failure)?;
                super::response::read(response, self.config.limit, admit_bad_request).await
            })
            .await
    }
}
pub(crate) fn failure(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Unavailable
    }
}
