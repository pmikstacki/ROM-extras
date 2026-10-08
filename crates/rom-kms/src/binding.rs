use rom_secrets::Error;
/// Distinct non-secret derivation context and authenticated associated data.
/// Hosts supply canonical bytes; neither input is an authentication credential.
pub struct Binding {
    context: Vec<u8>,
    aad: Vec<u8>,
}
impl Binding {
    /// Require nonempty context and at most 4,096 bytes in each input.
    pub fn new(context: Vec<u8>, aad: Vec<u8>) -> Result<Self, Error> {
        if context.len() > 4096 || aad.len() > 4096 {
            return Err(Error::Limit);
        }
        if context.is_empty() {
            return Err(Error::Invalid);
        }
        Ok(Self { context, aad })
    }
    /// Borrow the key derivation context, not the AAD.
    pub fn context(&self) -> &[u8] {
        &self.context
    }
    /// Borrow authenticated associated data, not the derivation context.
    pub fn aad(&self) -> &[u8] {
        &self.aad
    }
}
