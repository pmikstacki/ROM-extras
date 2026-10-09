//! Private transport configuration, separate from all browser descriptors.
use crate::AttributionProfile;
use rom_map_core::{Error, Result};
use std::time::Duration;
/// Operator-selected endpoint, provenance, attribution and private credential.
pub struct Config {
    http: rom_map_http::Config,
    provider: String,
    attribution: AttributionProfile,
    has_key: bool,
}
impl Config {
    /// Validate explicit service configuration without connecting or choosing a default service.
    pub fn new(
        endpoint: &str,
        agent: &str,
        provider: &str,
        interval: Duration,
        attribution: AttributionProfile,
    ) -> Result<Self> {
        if provider.len() > 128 {
            return Err(Error::TooLarge);
        }
        if provider.trim().is_empty() || provider.chars().any(char::is_control) {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            http: rom_map_http::Config::new(endpoint, agent, 1048576, interval, 1)?,
            provider: provider.into(),
            attribution,
            has_key: false,
        })
    }
    /// Supply a server-only API key; this never grants a token to the browser.
    pub fn with_server_key(mut self, key: String) -> Result<Self> {
        self.http = self.http.with_server_query_key("key", key)?;
        self.has_key = true;
        Ok(self)
    }
    /// Trust a host-approved private certificate authority.
    pub fn with_ca(mut self, ca: Vec<u8>) -> Result<Self> {
        self.http = self.http.with_ca(ca)?;
        Ok(self)
    }
}
/// Prepared private service transport; geocoding capabilities are added separately.
pub struct MapTiler {
    pub(crate) http: rom_map_http::Http,
    pub(crate) provider: String,
    pub(crate) attribution: AttributionProfile,
}
impl MapTiler {
    /// Prepare without network I/O; absent credentials fail before transport construction.
    pub fn new(config: Config) -> Result<Self> {
        if !config.has_key {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            http: rom_map_http::Http::new(config.http)?,
            provider: config.provider,
            attribution: config.attribution,
        })
    }
}
