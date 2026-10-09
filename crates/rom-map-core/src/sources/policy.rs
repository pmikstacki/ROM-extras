//! Explicit bounded host approval for browser network origins.
use crate::{Error, Result};
use url::{Origin, Url};
/// Explicit browser origins, without backend authentication or automatic requests.
pub struct BrowserPolicy {
    origins: Vec<Origin>,
}
impl BrowserPolicy {
    /// Require one through sixteen bare HTTPS origins without credentials, query or fragment.
    pub fn new(origins: &[&str]) -> Result<Self> {
        if origins.is_empty() {
            return Err(Error::InvalidQuery);
        }
        if origins.len() > 16 {
            return Err(Error::TooLarge);
        }
        let mut approved = Vec::new();
        for origin in origins {
            let url = public_url(origin)?;
            if url.path() != "/" || approved.contains(&url.origin()) {
                return Err(Error::InvalidQuery);
            }
            approved.push(url.origin());
        }
        Ok(Self { origins: approved })
    }
    /// Validate a public resource template without opening a connection or admitting query credentials.
    pub fn approve(&self, template: &str) -> Result<String> {
        // Parse the literal authority before substituting path placeholders.
        let url = public_url(template)?;
        let mut concrete = template.to_owned();
        for token in [
            "{z}",
            "{x}",
            "{y}",
            "{ratio}",
            "{quadkey}",
            "{bbox-epsg-3857}",
            "{fontstack}",
            "{range}",
        ] {
            concrete = concrete.replace(token, "0");
        }
        if concrete.contains(['{', '}']) {
            return Err(Error::Rejected);
        }
        if !self.origins.contains(&url.origin()) {
            return Err(Error::Rejected);
        }
        Ok(template.to_owned())
    }
    /// Exact approved origin set for host-selected browser connection controls.
    pub fn origins(&self) -> Vec<String> {
        self.origins
            .iter()
            .map(Origin::ascii_serialization)
            .collect()
    }
}
fn public_url(value: &str) -> Result<Url> {
    if value.len() > 4096 {
        return Err(Error::TooLarge);
    }
    if value.is_empty()
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
        || value.contains('\\')
    {
        return Err(Error::Rejected);
    }
    let url = Url::parse(value).map_err(|_| Error::Rejected)?;
    if url.scheme() != "https"
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Rejected);
    }
    Ok(url)
}
