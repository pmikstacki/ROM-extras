use crate::{Error, StrongEtag};
use reqwest::{
    Response,
    header::{self, HeaderMap, HeaderName},
};
fn single(headers: &HeaderMap, name: HeaderName) -> Option<&header::HeaderValue> {
    let mut values = headers.get_all(name).iter();
    let first = values.next()?;
    if values.next().is_some() {
        None
    } else {
        Some(first)
    }
}
fn mime(headers: &HeaderMap) -> bool {
    let Some(value) = single(headers, header::CONTENT_TYPE)
        .filter(|v| v.len() <= 128)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let lower = value.to_ascii_lowercase();
    let mut parts = lower.split(';');
    if parts.next().is_none_or(|p| p.trim() != "application/json") {
        return false;
    }
    let Some(parameter) = parts.next() else {
        return true;
    };
    matches!(parameter.trim(), "charset=utf-8" | "charset=\"utf-8\"") && parts.next().is_none()
}
pub(crate) async fn read(
    mut response: Response,
    etag: Option<&StrongEtag>,
    limit: usize,
) -> Result<Vec<u8>, Error> {
    match response.status().as_u16() {
        200 => {}
        404 => return Err(Error::Missing),
        412 => return Err(Error::PreconditionFailed),
        429 => {
            return Err(Error::RateLimited {
                retry_after_seconds: single(response.headers(), header::RETRY_AFTER)
                    .filter(|v| v.len() <= 16)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u32>().ok())
                    .filter(|v| *v <= 3600),
            });
        }
        500..=599 => return Err(Error::Unavailable),
        _ => return Err(Error::Rejected),
    }
    let retained = response
        .headers()
        .iter()
        .try_fold(0usize, |sum, (name, value)| {
            sum.checked_add(name.as_str().len() + value.len())
        });
    if retained.is_none_or(|size| size > 8192) {
        return Err(Error::TooLarge);
    }
    if !mime(response.headers()) {
        return Err(Error::Rejected);
    }
    if response.headers().contains_key(header::CONTENT_ENCODING)
        && single(response.headers(), header::CONTENT_ENCODING)
            .is_none_or(|v| v.as_bytes() != b"identity")
    {
        return Err(Error::Rejected);
    }
    if let Some(etag) = etag
        && single(response.headers(), header::ETAG)
            .is_none_or(|actual| actual.as_bytes() != etag.0.as_bytes())
    {
        return Err(Error::PreconditionFailed);
    }
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(Error::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(crate::http::failure)? {
        if chunk.len() > limit - body.len() {
            return Err(Error::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
