//! Explicit bounded host approval for browser network origins.
use crate::{Error, Result};
use url::{Origin, Url};
/// Explicit browser origins, without backend authentication or automatic requests.
pub struct BrowserPolicy {
    origins: Vec<Origin>,
    public_tokens: Vec<PublicQueryToken>,
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
        Ok(Self {
            origins: approved,
            public_tokens: Vec::new(),
        })
    }
    /// Validate a public resource template without opening a connection or admitting query credentials.
    pub fn approve(&self, template: &str) -> Result<String> {
        // Parse the literal authority before substituting path placeholders.
        let url = resource_url(template, true)?;
        if url.query().is_some() {
            let pairs = url.query_pairs().collect::<Vec<_>>();
            if pairs.len() != 1
                || !self.public_tokens.iter().any(|grant| {
                    grant.origin == url.origin()
                        && grant.field == pairs[0].0
                        && grant.token == pairs[0].1
                })
            {
                return Err(Error::Rejected);
            }
        }
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
    /// Explicitly approve an origin-restricted token intended for browser disclosure by the host.
    /// This never obtains a backend credential. The host must configure provider-side app-origin restrictions.
    pub fn with_public_query_token(
        mut self,
        origin: &str,
        field: &str,
        token: String,
    ) -> Result<Self> {
        let url = public_url(origin)?;
        if url.path() != "/" || !self.origins.contains(&url.origin()) {
            return Err(Error::Rejected);
        }
        if field.is_empty()
            || field.len() > 128
            || !field
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            || token.is_empty()
            || token.len() > 1024
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'~'))
        {
            return Err(Error::InvalidQuery);
        }
        if self
            .public_tokens
            .iter()
            .any(|grant| grant.origin == url.origin())
        {
            return Err(Error::InvalidQuery);
        }
        self.public_tokens.push(PublicQueryToken {
            origin: url.origin(),
            field: field.into(),
            token,
        });
        Ok(self)
    }
    /// Exact approved origin set for host-selected browser connection controls.
    pub fn origins(&self) -> Vec<String> {
        self.origins
            .iter()
            .map(Origin::ascii_serialization)
            .collect()
    }
}
struct PublicQueryToken {
    origin: Origin,
    field: String,
    token: String,
}
fn public_url(value: &str) -> Result<Url> {
    resource_url(value, false)
}
fn resource_url(value: &str, allow_query: bool) -> Result<Url> {
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
        || (!allow_query && url.query().is_some())
        || url.fragment().is_some()
    {
        return Err(Error::Rejected);
    }
    Ok(url)
}
