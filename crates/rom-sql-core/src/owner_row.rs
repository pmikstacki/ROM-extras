use crate::{OwnerError, OwnerState, OwnerToken};

/// Validate already-decoded native ownership fields without repairing persisted data.
/// Providers must first enforce their SDK's exact integer and binary wire types.
/// The separate token length detects malformed bytes hidden by a bounded SQL projection.
/// Inspection of the returned state does not grant ownership.
pub fn validate_owner_row(
    format: i64,
    expected_identity: &[u8; 32],
    actual_identity: Option<&[u8]>,
    generation: i64,
    token: Option<&[u8]>,
    token_length: Option<i64>,
) -> Result<OwnerState, OwnerError> {
    if format != 1 {
        return Err(OwnerError::UnsupportedFormat);
    }
    if actual_identity != Some(&expected_identity[..]) {
        return Err(OwnerError::Invalid);
    }
    let generation = u64::try_from(generation).map_err(|_| OwnerError::Invalid)?;
    let token = match (token, token_length) {
        (None, None) => None,
        (Some(bytes), Some(32)) => Some(OwnerToken::new(
            bytes.try_into().map_err(|_| OwnerError::Invalid)?,
        )?),
        _ => return Err(OwnerError::Invalid),
    };
    OwnerState::new(generation, token)
}
