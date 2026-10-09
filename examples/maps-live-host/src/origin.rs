use axum::http::{HeaderMap, StatusCode, Uri};
use rom_map_core::{Error, Result};
pub(crate) fn admit(origin: &str) -> Result<String> {
    let uri: Uri = origin.parse().map_err(|_| Error::InvalidQuery)?;
    if origin.len() > 256
        || !matches!(uri.scheme_str(), Some("http" | "https"))
        || uri.authority().is_none()
        || uri.authority().is_some_and(|a| a.as_str().contains('@'))
        || uri.path() != "/"
        || uri.query().is_some()
    {
        return Err(Error::InvalidQuery);
    }
    Ok(origin.into())
}
pub(crate) fn check(headers: &HeaderMap, origin: &str) -> std::result::Result<(), StatusCode> {
    if headers.get_all("origin").iter().count() != 1
        || headers.get("origin").and_then(|v| v.to_str().ok()) != Some(origin)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}
