use crate::{Error, HttpConfig, RequestContext, StrongEtag};
use rom_import::{ActionPlan, PreparedAction};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, Semaphore},
    time::Instant,
};
/// Explicit conditional source reader. Construction performs no network request.
pub struct HttpSource {
    config: HttpConfig,
    client: reqwest::Client,
    admission: Arc<Semaphore>,
    next: Mutex<Option<Instant>>,
}
impl HttpSource {
    /// Configure TLS and finite read admission without activating a request.
    pub fn new(config: HttpConfig) -> Result<Self, Error> {
        let mut builder = reqwest::Client::builder()
            .tls_backend_rustls()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .http1_only()
            .http1_max_headers(32)
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(60))
            .user_agent(&config.agent);
        if let Some(ca) = &config.ca {
            builder = builder.tls_certs_only(vec![
                reqwest::Certificate::from_pem(ca).map_err(|_| Error::InvalidConfig)?,
            ]);
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
    /// Fetch and verify exact JSON bytes under one total deadline, without mutation or retry.
    /// Conditional reads require the exact strong ETag echoed by HTTP200.
    pub async fn fetch(
        &self,
        plan: &ActionPlan,
        etag: Option<&StrongEtag>,
        context: &RequestContext,
    ) -> Result<PreparedAction, Error> {
        context
            .run(async {
                let _slot = self
                    .admission
                    .clone()
                    .try_acquire_owned()
                    .map_err(|_| Error::Overloaded)?;
                {
                    let mut next = self.next.lock().await;
                    if let Some(ready) = *next {
                        tokio::time::sleep_until(ready).await;
                    }
                    *next = Some(Instant::now() + self.config.interval);
                }
                let mut request = self
                    .client
                    .get(self.config.endpoint.clone())
                    .header(reqwest::header::ACCEPT, "application/json")
                    .header(reqwest::header::ACCEPT_ENCODING, "identity");
                if let Some(token) = &self.config.bearer {
                    let mut value = reqwest::header::HeaderValue::from_str(&format!(
                        "Bearer {}",
                        token.as_str()
                    ))
                    .map_err(|_| Error::InvalidConfig)?;
                    value.set_sensitive(true);
                    request = request.header(reqwest::header::AUTHORIZATION, value);
                }
                if let Some(etag) = etag {
                    request = request.header(reqwest::header::IF_MATCH, &etag.0);
                }
                let response = request.send().await.map_err(failure)?;
                let bytes =
                    crate::response::read(response, etag, plan.max_document_bytes()).await?;
                plan.prepare(&bytes).map_err(Error::Admission)
            })
            .await
    }
}
impl std::fmt::Debug for HttpSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HttpSource { redacted }")
    }
}
pub(crate) fn failure(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Unavailable
    }
}
