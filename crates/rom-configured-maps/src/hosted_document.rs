use rom_map_core::{Error, RequestContext, Result};
pub(crate) struct HostedDocument {
    http: rom_map_http::Http,
    name: String,
}
impl HostedDocument {
    pub(crate) fn new(http: rom_map_http::Config, name: &str) -> Result<Self> {
        if name.is_empty()
            || name.len() > 256
            || matches!(name, "." | "..")
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            http: rom_map_http::Http::new(http)?,
            name: name.into(),
        })
    }
    pub(crate) async fn bytes(&self, context: &RequestContext) -> Result<Vec<u8>> {
        self.http.get(&[&self.name], &[], context).await
    }
}
