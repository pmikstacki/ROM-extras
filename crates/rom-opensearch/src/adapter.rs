//! Owned fixed-generation projection adapter.
use crate::{TlsConfig, mapping, transport::Transport, writes};
use rom_projection_core::{ProjectionProfile, Result};
/// Trusted fixed-generation target. Methods never accept raw DSL or arbitrary request paths.
pub struct OpenSearch {
    pub(crate) transport: Transport,
    pub(crate) profile: ProjectionProfile,
    pub(crate) physical: String,
    pub(crate) profile_digest: String,
    pub(crate) text_fields: Vec<String>,
    pub(crate) index_uuid: Option<String>,
}
impl OpenSearch {
    /// Fix verified TLS, the derived document profile, physical generation and indexed text fields.
    /// Selected non-text values remain stored without indexing; indexed fields admit strings or null.
    pub fn new(
        config: TlsConfig,
        profile: ProjectionProfile,
        physical: &str,
        text_fields: Vec<String>,
    ) -> Result<Self> {
        mapping::identifier(physical)?;
        let text_fields = mapping::fields(text_fields)?;
        let profile_digest = writes::hex(&profile.fingerprint());
        Ok(Self {
            transport: Transport::new(config)?,
            profile,
            physical: physical.into(),
            profile_digest,
            text_fields,
            index_uuid: None,
        })
    }
}
