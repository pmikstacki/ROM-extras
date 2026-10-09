use crate::DeliveryError;
/// Validate the existing stable delivery wire identity before encoding or external I/O.
/// Accept 1–2048 ASCII letters, digits, hyphens or underscores; never normalize it.
pub fn validate_delivery_identity(id: &str) -> Result<(), DeliveryError> {
    if id.is_empty()
        || id.len() > 2048
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(DeliveryError::InvalidIdentity);
    }
    Ok(())
}
