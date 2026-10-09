use rom_map_core::{Attribution, BrowserPolicy, Result, TileKind, TileSource};
pub(crate) struct TileDocument {
    bytes: Vec<u8>,
    policy: BrowserPolicy,
    credit: Attribution,
    kind: TileKind,
}
impl TileDocument {
    pub(crate) fn new(
        bytes: &[u8],
        policy: BrowserPolicy,
        credit: Attribution,
        kind: TileKind,
    ) -> Result<Self> {
        Self::approve(bytes, &policy, &credit, kind)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            policy,
            credit,
            kind,
        })
    }
    fn approve(
        bytes: &[u8],
        policy: &BrowserPolicy,
        credit: &Attribution,
        kind: TileKind,
    ) -> Result<TileSource> {
        TileSource::from_json(
            bytes,
            kind,
            policy,
            Attribution::new(credit.text(), credit.link())?,
        )
    }
    pub(crate) fn source(&self) -> Result<TileSource> {
        Self::approve(&self.bytes, &self.policy, &self.credit, self.kind)
    }
}
