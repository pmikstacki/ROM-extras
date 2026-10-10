use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rom_auth::AuthError;
pub(crate) fn validate(n: &str, e: &str) -> Result<(), AuthError> {
    let n = unsigned(n, 1366)?;
    let bits = n.len() * 8 - n[0].leading_zeros() as usize;
    if !(2048..=8192).contains(&bits) || n[n.len() - 1] & 1 == 0 {
        return Err(AuthError::Invalid);
    }
    let e = unsigned(e, 6)?;
    if e.len() > 4 {
        return Err(AuthError::Invalid);
    }
    let value = e.iter().fold(0u32, |a, b| (a << 8) | u32::from(*b));
    if value < 3 || value & 1 == 0 {
        return Err(AuthError::Invalid);
    }
    Ok(())
}
fn unsigned(value: &str, max: usize) -> Result<Vec<u8>, AuthError> {
    if value.len() > max {
        return Err(AuthError::TooLarge);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| AuthError::Invalid)?;
    if bytes.is_empty() || bytes[0] == 0 || URL_SAFE_NO_PAD.encode(&bytes) != value {
        return Err(AuthError::Invalid);
    }
    Ok(bytes)
}
