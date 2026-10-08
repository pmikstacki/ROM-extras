use rom_secrets::{Error, SecretRef};
fn segment(value: &str) -> Result<String, Error> {
    SecretRef::new(value).map(|r| r.as_str().to_owned())
}
/// Host-approved KV v2 location; no URL or arbitrary Resource path.
pub struct SecretLocation {
    pub(crate) mount: String,
    pub(crate) path: Vec<String>,
    pub(crate) field: String,
}
impl SecretLocation {
    /// Validate mount, up to 16 path segments, and a string field selector.
    pub fn new(mount: &str, path: &str, field: &str) -> Result<Self, Error> {
        if path.len() > 2048 {
            return Err(Error::Limit);
        }
        let path: Vec<_> = path.split('/').map(segment).collect::<Result<_, _>>()?;
        if path.len() > 16 {
            return Err(Error::Limit);
        }
        Ok(Self {
            mount: segment(mount)?,
            path,
            field: segment(field)?,
        })
    }
}
/// Host-approved existing derived Transit key location.
pub struct KeyLocation {
    pub(crate) mount: String,
    pub(crate) key: String,
}
impl KeyLocation {
    /// Validate one mount and existing key name. Does not provision a key.
    pub fn new(mount: &str, key: &str) -> Result<Self, Error> {
        Ok(Self {
            mount: segment(mount)?,
            key: segment(key)?,
        })
    }
}
