use crate::EmailError;
use lettre::Address;
pub(crate) fn mailbox(s: &str) -> Result<Address, EmailError> {
    if s.is_empty()
        || s.len() > 254
        || !s.is_ascii()
        || s.bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
    {
        return Err(EmailError::InvalidPayload);
    }
    let (local, _) = s.rsplit_once('@').ok_or(EmailError::InvalidPayload)?;
    if local.len() > 64 {
        return Err(EmailError::InvalidPayload);
    }
    s.parse().map_err(|_| EmailError::InvalidPayload)
}
pub(crate) fn domain(s: &str) -> bool {
    s.len() <= 253
        && s.contains('.')
        && s.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 63
                && !part.starts_with('-')
                && !part.ends_with('-')
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}
