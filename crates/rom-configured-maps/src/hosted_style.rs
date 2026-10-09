use crate::{ConfiguredStyle, hosted_document::HostedDocument};
use rom_map_core::{
    Attribution, BrowserPolicy, Error, MapStyle, ProviderFuture, RequestContext, Result, Styles,
    TileSource,
};
/// Host-selected HTTPS style; TileJSON references require explicit pre-approved source bindings.
pub struct HostedStyle {
    document: HostedDocument,
    policy: BrowserPolicy,
    credits: Vec<Attribution>,
    resolved: Vec<(String, TileSource)>,
}
impl HostedStyle {
    /// Prepare an explicit document and dependencies without network activity.
    pub fn new(
        http: rom_map_http::Config,
        document: &str,
        policy: BrowserPolicy,
        credits: Vec<Attribution>,
        resolved: Vec<(String, TileSource)>,
    ) -> Result<Self> {
        if credits.is_empty() {
            return Err(Error::InvalidQuery);
        }
        if credits.len() > 16 || resolved.len() > 128 {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            document: HostedDocument::new(http, document)?,
            policy,
            credits,
            resolved,
        })
    }
}
impl Styles for HostedStyle {
    fn style<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, MapStyle> {
        Box::pin(async move {
            context
                .run(async {
                    let bytes = self.document.bytes(context).await?;
                    ConfiguredStyle::approve(&bytes, &self.policy, &self.credits, &self.resolved)
                })
                .await
        })
    }
}
