use crate::OpenBao;
use reqwest::Method;
use rom_secrets::{Error, ResolvedSecret, SecretBytes, SecretRef, SecretResolver, Version};
impl SecretResolver for OpenBao {
    async fn resolve(
        &self,
        reference: &SecretRef,
        version: Version,
    ) -> Result<ResolvedSecret, Error> {
        let location = self.secrets.get(reference).ok_or(Error::Invalid)?;
        let _permit = self.transport.admit()?;
        let mut segments = vec![location.mount.as_str(), "data"];
        segments.extend(location.path.iter().map(String::as_str));
        let mut url = self.transport.url(&segments)?;
        if let Version::Pinned(version) = version {
            url.query_pairs_mut()
                .append_pair("version", &version.to_string());
        }
        let mut response = self
            .transport
            .request(Method::GET, url, None, Error::Invalid)
            .await?;
        let actual = response
            .pointer("/data/metadata/version")
            .and_then(|v| v.as_u64())
            .filter(|v| *v > 0)
            .ok_or(Error::Protocol)?;
        if matches!(version,Version::Pinned(wanted) if wanted.get()!=actual) {
            return Err(Error::Protocol);
        }
        let value = response
            .pointer_mut("/data/data")
            .and_then(|v| v.as_object_mut())
            .and_then(|v| v.remove(&location.field))
            .ok_or(Error::NotFound)?;
        let serde_json::Value::String(value) = value else {
            return Err(Error::Protocol);
        };
        if value.is_empty() {
            return Err(Error::Protocol);
        }
        ResolvedSecret::new(SecretBytes::new(value.into_bytes())?, actual)
    }
}
