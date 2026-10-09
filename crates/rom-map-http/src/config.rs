//! Explicit host endpoint and finite admission configuration.
use rom_map_core::{Error, Result};
use std::time::Duration;
use url::Url;
use zeroize::Zeroizing;
/// Explicit host configuration; no network connection starts during validation.
pub struct Config {
    pub(crate) endpoint: Url,
    pub(crate) agent: String,
    pub(crate) limit: usize,
    pub(crate) interval: Duration,
    pub(crate) concurrency: usize,
    pub(crate) ca: Option<Vec<u8>>,
    pub(crate) key: Option<(String, Zeroizing<String>)>,
}
impl Config {
    /// Configure a fixed endpoint, identifying user agent and bounded admission.
    pub fn new(
        endpoint: &str,
        agent: &str,
        limit: usize,
        interval: Duration,
        concurrency: usize,
    ) -> Result<Self> {
        if endpoint.len() > 2048 || agent.len() > 512 || limit > 1048576 {
            return Err(Error::TooLarge);
        }
        if limit == 0
            || !(1..=8).contains(&concurrency)
            || interval > Duration::from_secs(60)
            || agent.trim().is_empty()
            || reqwest::header::HeaderValue::from_str(agent).is_err()
            || endpoint
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            || endpoint.contains('\\')
        {
            return Err(Error::InvalidQuery);
        }
        let raw = endpoint.to_ascii_lowercase();
        if ["%2e", "%2f", "%5c", "/../", "/./"]
            .iter()
            .any(|value| raw.contains(value))
        {
            return Err(Error::InvalidQuery);
        }
        let endpoint = Url::parse(endpoint).map_err(|_| Error::InvalidQuery)?;
        if endpoint.scheme() != "https"
            || endpoint.host().is_none()
            || !endpoint.path().ends_with('/')
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            endpoint,
            agent: agent.into(),
            limit,
            interval,
            concurrency,
            ca: None,
            key: None,
        })
    }
    /// Replace default trust roots with one bounded host-approved PEM certificate.
    pub fn with_ca(mut self, ca: Vec<u8>) -> Result<Self> {
        if ca.len() > 32768 {
            return Err(Error::TooLarge);
        }
        if ca.is_empty() || reqwest::Certificate::from_pem(&ca).is_err() {
            return Err(Error::InvalidQuery);
        }
        self.ca = Some(ca);
        Ok(self)
    }
    /// Keep a bounded server credential private; this does not create a browser grant.
    pub fn with_server_query_key(mut self, field: &str, value: String) -> Result<Self> {
        let value = Zeroizing::new(value);
        if value.len() > 4096 || field.len() > 128 {
            return Err(Error::TooLarge);
        }
        if value.is_empty()
            || value.chars().any(char::is_control)
            || field.is_empty()
            || !field
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(Error::InvalidQuery);
        }
        self.key = Some((field.into(), value));
        Ok(self)
    }
}
