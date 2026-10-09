use crate::Error;
use std::time::Duration;
use url::Url;
use zeroize::Zeroizing;
/// Fixed server-only acquisition configuration. No browser contract or environment discovery.
pub struct HttpConfig {
    pub(crate) endpoint: Url,
    pub(crate) agent: String,
    pub(crate) interval: Duration,
    pub(crate) concurrency: usize,
    pub(crate) ca: Option<Vec<u8>>,
    pub(crate) bearer: Option<Zeroizing<String>>,
}
impl HttpConfig {
    /// Validate a fixed HTTPS URL, agent, 0–60s start interval and 1–8 operation slots.
    /// Query credentials, fragments, userinfo and ambiguous paths are refused.
    pub fn new(
        endpoint: &str,
        agent: &str,
        interval: Duration,
        concurrency: usize,
    ) -> Result<Self, Error> {
        if endpoint.len() > 2048
            || agent.len() > 512
            || agent.trim().is_empty()
            || reqwest::header::HeaderValue::from_str(agent).is_err()
            || interval > Duration::from_secs(60)
            || !(1..=8).contains(&concurrency)
            || endpoint
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
            || endpoint.contains('\\')
        {
            return Err(Error::InvalidConfig);
        }
        let lower = endpoint.to_ascii_lowercase();
        if ["%2e", "%2f", "%5c", "/../", "/./"]
            .iter()
            .any(|bad| lower.contains(bad))
            || lower.ends_with("/.")
            || lower.ends_with("/..")
        {
            return Err(Error::InvalidConfig);
        }
        let endpoint = Url::parse(endpoint).map_err(|_| Error::InvalidConfig)?;
        if endpoint.scheme() != "https"
            || endpoint.host().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            endpoint,
            agent: agent.into(),
            interval,
            concurrency,
            ca: None,
            bearer: None,
        })
    }
    /// Replace default TLS trust with one explicit bounded private CA certificate.
    pub fn with_ca(mut self, ca: Vec<u8>) -> Result<Self, Error> {
        if ca.is_empty() || ca.len() > 32768 || reqwest::Certificate::from_pem(&ca).is_err() {
            return Err(Error::InvalidConfig);
        }
        self.ca = Some(ca);
        Ok(self)
    }
    /// Retain an explicit server-only bearer token, at most4096 ASCII token bytes.
    /// No claim is made that every HTTP library allocation of a header is zeroized.
    pub fn with_bearer(mut self, token: String) -> Result<Self, Error> {
        let token = Zeroizing::new(token);
        if token.is_empty()
            || token.len() > 4096
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~+/=".contains(&b))
        {
            return Err(Error::InvalidConfig);
        }
        self.bearer = Some(token);
        Ok(self)
    }
}
impl std::fmt::Debug for HttpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HttpConfig { redacted }")
    }
}
