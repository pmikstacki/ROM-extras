use crate::Error;
use zeroize::Zeroizing;
/// Owned bounded bytes with explicit exposure and zeroization on drop.
///
/// This type has no Serialize, Debug, Clone or Copy implementation. Copies in
/// HTTP, parser, operating-system or host buffers are not erased by its drop.
///
/// ```compile_fail
/// use rom_secrets::SecretBytes;
/// let bytes = SecretBytes::new(vec![1]).unwrap();
/// println!("{bytes:?}");
/// ```
/// ```compile_fail
/// use rom_secrets::SecretBytes;
/// let bytes = SecretBytes::new(vec![1]).unwrap();
/// let copied = bytes.clone();
/// ```
pub struct SecretBytes(Zeroizing<Vec<u8>>);
impl SecretBytes {
    /// Take ownership of at most 4,096 bytes. Empty KMS plaintext is permitted.
    /// Oversized owned bytes are cleared before this method returns an error.
    pub fn new(bytes: Vec<u8>) -> Result<Self, Error> {
        let bytes = Zeroizing::new(bytes);
        if bytes.len() > 4096 {
            return Err(Error::Limit);
        }
        Ok(Self(bytes))
    }
    /// Explicitly borrow bytes for a host operation. Do not persist or log them.
    pub fn expose(&self) -> &[u8] {
        self.0.as_slice()
    }
}
/// Nonempty resolved credential and its actual positive provider version.
pub struct ResolvedSecret {
    bytes: SecretBytes,
    version: u64,
}
impl ResolvedSecret {
    /// Require nonempty material and an actual positive provider version.
    pub fn new(bytes: SecretBytes, version: u64) -> Result<Self, Error> {
        if bytes.expose().is_empty() || version == 0 {
            return Err(Error::Invalid);
        }
        Ok(Self { bytes, version })
    }
    /// Borrow protected material for explicit host activation.
    pub fn bytes(&self) -> &SecretBytes {
        &self.bytes
    }
    /// Return the version actually resolved by the provider.
    pub fn version(&self) -> u64 {
        self.version
    }
}
