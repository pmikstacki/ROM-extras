//! Explicit browser-intended token disclosure, independent of server configuration.
use rom_map_core::{BrowserPolicy, Result};
/// Host-approved browser token and service origins; cannot obtain a backend key.
pub struct MapBrowserGrant {
    pub(crate) policy: BrowserPolicy,
    pub(crate) token: String,
}
impl MapBrowserGrant {
    /// Approve one browser-intended key for each explicit HTTPS service origin.
    /// The host must set application-origin restrictions in its provider account.
    pub fn new(origins: &[&str], token: String) -> Result<Self> {
        let mut policy = BrowserPolicy::new(origins)?;
        for origin in origins {
            policy = policy.with_public_query_token(origin, "key", token.clone())?;
        }
        Ok(Self { policy, token })
    }
}
