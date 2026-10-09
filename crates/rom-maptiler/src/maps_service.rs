//! Shared server transport and explicit metadata-processing permission.
use crate::MapMetadata;
use rom_map_core::{Error, Result};
use std::time::Duration;
/// Private backend selection with no default service or browser activation.
pub struct MapsConfig {
    http: rom_map_http::Config,
    metadata: MapMetadata,
    permission: bool,
}
impl MapsConfig {
    /// Prepare one bounded backend reader and an independently approved browser descriptor policy.
    pub fn new(
        endpoint: &str,
        agent: &str,
        interval: Duration,
        metadata: MapMetadata,
    ) -> Result<Self> {
        Ok(Self {
            http: rom_map_http::Config::new(endpoint, agent, 1048576, interval, 1)?
                .with_server_query_key("key", metadata.server_key.to_string())?,
            metadata,
            permission: false,
        })
    }
    /// Explicit host declaration that the endpoint/account agreement permits backend metadata processing.
    /// This does not verify provider-side permissions or permit caching, tile proxying or redistribution.
    pub fn with_backend_metadata_permission(mut self) -> Self {
        self.permission = true;
        self
    }
    /// Replace trust roots with one host-approved private CA; never disable verification.
    pub fn with_ca(mut self, ca: Vec<u8>) -> Result<Self> {
        self.http = self.http.with_ca(ca)?;
        Ok(self)
    }
}
/// Shared map-metadata service with aggregate admission and rate limits across its adapters.
pub struct MapTilerMaps {
    pub(crate) http: rom_map_http::Http,
    pub(crate) metadata: MapMetadata,
}
impl MapTilerMaps {
    /// Construct without I/O only after the host declares its permitted metadata use.
    /// Share this instance through Arc for aggregate admission across capabilities.
    pub fn new(config: MapsConfig) -> Result<Self> {
        if !config.permission {
            return Err(Error::Rejected);
        }
        Ok(Self {
            http: rom_map_http::Http::new(config.http)?,
            metadata: config.metadata,
        })
    }
}
pub(crate) fn service_id(id: &str) -> Result<String> {
    if id.len() > 128 {
        return Err(Error::TooLarge);
    }
    if id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(Error::InvalidQuery);
    }
    Ok(id.into())
}
