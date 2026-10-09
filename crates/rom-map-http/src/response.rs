//! Closed response failures and finite declared/streamed payload admission.
use rom_map_core::{Error, Result};
pub(crate) async fn read(
    mut response: reqwest::Response,
    limit: usize,
    admit_bad_request: bool,
) -> Result<crate::NativeResponse> {
    let status = response.status();
    if status.as_u16() == 429 {
        let retry_after_seconds = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|v| *v <= 3600);
        return Err(Error::RateLimited {
            retry_after_seconds,
        });
    }
    if !status.is_success() && !(admit_bad_request && status.as_u16() == 400) {
        return Err(if status.is_server_error() {
            Error::Unavailable
        } else {
            Error::Rejected
        });
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(Error::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(super::transport::failure)? {
        if chunk.len() > limit - body.len() {
            return Err(Error::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(crate::NativeResponse {
        status: if status.as_u16() == 400 {
            crate::ReadStatus::BadRequest
        } else {
            crate::ReadStatus::Success
        },
        body,
    })
}
