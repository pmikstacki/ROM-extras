//! Fixed verified TLS origin, private authentication and finite operation deadline.
use rom_projection_core::{Error, Result};
use std::time::Duration;
use url::Url;
use zeroize::Zeroizing;
pub(crate) enum Authentication {
    Identity(Zeroizing<Vec<u8>>),
    ApiKey(Zeroizing<Vec<u8>>),
}
/// Owned TLS configuration. Credentials and origins have no Debug implementation.
pub struct TlsConfig {
    pub(crate) origin: Url,
    pub(crate) ca: Vec<u8>,
    pub(crate) authentication: Authentication,
    pub(crate) deadline: Duration,
}
impl TlsConfig {
    /// Require a bare HTTPS origin, bounded CA and PEM client identity, and a 1-ms through 60-s deadline.
    pub fn new(endpoint: &str, ca: Vec<u8>, identity: Vec<u8>, deadline: Duration) -> Result<Self> {
        let identity = Zeroizing::new(identity);
        if identity.len() > 32768 {
            return Err(Error::TooLarge);
        }
        if identity.is_empty() {
            return Err(Error::Invalid);
        }
        Self::assemble(endpoint, ca, Authentication::Identity(identity), deadline)
    }
    /// Require a bounded opaque API key, bare HTTPS origin, trusted CA and finite deadline.
    pub fn api_key(endpoint: &str, ca: Vec<u8>, key: Vec<u8>, deadline: Duration) -> Result<Self> {
        let key = Zeroizing::new(key);
        if key.len() > 4096 {
            return Err(Error::TooLarge);
        }
        if key.is_empty() || reqwest::header::HeaderValue::from_bytes(&key).is_err() {
            return Err(Error::Invalid);
        }
        Self::assemble(endpoint, ca, Authentication::ApiKey(key), deadline)
    }
    fn assemble(
        endpoint: &str,
        ca: Vec<u8>,
        authentication: Authentication,
        deadline: Duration,
    ) -> Result<Self> {
        if endpoint.len() > 2048 || ca.len() > 32768 {
            return Err(Error::TooLarge);
        }
        if ca.is_empty()
            || !(Duration::from_millis(1)..=Duration::from_secs(60)).contains(&deadline)
        {
            return Err(Error::Invalid);
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
        Ok(Self {
            origin,
            ca,
            authentication,
            deadline,
        })
    }
}
