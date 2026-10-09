use rom_map_core::{
    Attribution, BrowserPolicy, MapStyle, ProviderFuture, RequestContext, Result, Styles,
    TileSource,
};
/// Explicit host style with approved, already resolved TileJSON references.
pub struct ConfiguredStyle {
    bytes: Vec<u8>,
    policy: BrowserPolicy,
    credits: Vec<Attribution>,
    resolved: Vec<(String, TileSource)>,
}
impl ConfiguredStyle {
    /// Validate a style and dependencies keyed by exact native source identifier.
    /// No nested URLs are fetched.
    pub fn new(
        bytes: &[u8],
        policy: BrowserPolicy,
        credits: Vec<Attribution>,
        resolved: Vec<(String, TileSource)>,
    ) -> Result<Self> {
        Self::approve(bytes, &policy, &credits, &resolved)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            policy,
            credits,
            resolved,
        })
    }
    pub(crate) fn approve(
        bytes: &[u8],
        policy: &BrowserPolicy,
        credits: &[Attribution],
        resolved: &[(String, TileSource)],
    ) -> Result<MapStyle> {
        let refs = resolved
            .iter()
            .map(|(id, source)| (id.as_str(), source))
            .collect::<Vec<_>>();
        MapStyle::from_json(bytes, policy, credits, &refs)
    }
}
impl Styles for ConfiguredStyle {
    fn style<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, MapStyle> {
        Box::pin(async move {
            context
                .run(async {
                    Self::approve(&self.bytes, &self.policy, &self.credits, &self.resolved)
                })
                .await
        })
    }
}
