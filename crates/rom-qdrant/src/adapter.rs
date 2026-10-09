//! Fixed-origin writer; credentials remain inside the shared transport.
use crate::Generation;
use rom_projection_core::Result;
use rom_projection_http::{Http, TlsConfig};
/// Standalone fixed-generation target using a host-issued collection-scoped `prw` JWT.
/// The host must prevent aliasing, name reuse and administrative restoration during pending work.
/// Dropping a future stops local waiting; it does not establish cancellation of native writes.
pub struct Qdrant {
    pub(crate) http: Http,
    pub(crate) generation: Generation,
}
impl Qdrant {
    /// Build a writer with explicit verified TLS and fixed generation. No network operation occurs.
    /// Credential scope is enforced by the native server and must be qualified by the host.
    pub fn new(config: TlsConfig, generation: Generation) -> Result<Self> {
        Ok(Self {
            http: Http::new(config)?,
            generation,
        })
    }
}
