use crate::Error;
/// One strong visible-ASCII quoted entity-tag; no wildcard, list or weak syntax.
/// A validator does not authenticate a producer or replace the expected digest.
pub struct StrongEtag(pub(crate) String);
impl StrongEtag {
    /// Validate the narrower host import profile, at most256 bytes including quotes.
    pub fn new(value: &str) -> Result<Self, Error> {
        let bytes = value.as_bytes();
        if !(2..=256).contains(&bytes.len())
            || bytes.first() != Some(&b'"')
            || bytes.last() != Some(&b'"')
            || !bytes[1..bytes.len() - 1]
                .iter()
                .all(|b| (0x21..=0x7e).contains(b) && *b != b'"')
        {
            return Err(Error::InvalidConfig);
        }
        Ok(Self(value.into()))
    }
}
impl std::fmt::Debug for StrongEtag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StrongEtag { redacted }")
    }
}
