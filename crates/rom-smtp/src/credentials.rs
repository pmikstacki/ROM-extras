use crate::SmtpError;
use std::fmt;
use zeroize::Zeroizing;
/// Server-only authentication values with zeroized owned storage and opaque Debug.
/// Copies in the external protocol library are not guaranteed to be zeroized.
pub struct Credentials {
    pub(crate) username: Zeroizing<String>,
    pub(crate) password: Zeroizing<String>,
}
impl Credentials {
    /// Validate 1–1024 UTF-8 bytes per value, with no control characters.
    pub fn new(username: String, password: String) -> Result<Self, SmtpError> {
        let username = Zeroizing::new(username);
        let password = Zeroizing::new(password);
        if [&*username, &*password]
            .iter()
            .any(|v| v.is_empty() || v.len() > 1024 || v.chars().any(char::is_control))
        {
            return Err(SmtpError::InvalidCredentials);
        }
        Ok(Self { username, password })
    }
}
impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials").finish_non_exhaustive()
    }
}
