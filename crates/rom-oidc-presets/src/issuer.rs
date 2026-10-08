use rom_auth::{AuthError, OidcIdTokenAdapter, jwt::TrustedKeys};
use url::Url;

/// Host-selected exact HTTPS issuer for ROM's existing human ID-token verifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssuerPreset {
    issuer: String,
}

impl IssuerPreset {
    /// Retain an exact HTTPS issuer, including its path and trailing slash.
    ///
    /// Reject credentials, query, fragment, whitespace, controls, and backslashes.
    /// Input must use an explicit `https://` authority and fit within 2048 bytes.
    /// Validation never normalizes the string used for token issuer comparison.
    pub fn exact_https(issuer: &str) -> Result<Self, AuthError> {
        if issuer.len() > 2048
            || issuer
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
        {
            return Err(AuthError::InvalidConfiguration);
        }
        let authority = issuer
            .strip_prefix("https://")
            .and_then(|rest| rest.split('/').next())
            .filter(|value| !value.is_empty() && !value.contains('@'))
            .ok_or(AuthError::InvalidConfiguration)?;
        let parsed = Url::parse(issuer).map_err(|_| AuthError::InvalidConfiguration)?;
        if authority.is_empty()
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(AuthError::InvalidConfiguration);
        }
        Ok(Self {
            issuer: issuer.into(),
        })
    }

    /// Append Keycloak's realm path under an explicit HTTPS deployment base.
    ///
    /// Realm names use 1..=256 ASCII unreserved bytes, excluding `.` and `..`.
    /// Other realm names require an explicit issuer with [`Self::exact_https`].
    pub fn keycloak(base: &str, realm: &str) -> Result<Self, AuthError> {
        Self::exact_https(base)?;
        if realm.is_empty()
            || realm.len() > 256
            || matches!(realm, "." | "..")
            || !realm
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
        {
            return Err(AuthError::InvalidConfiguration);
        }
        Self::exact_https(&format!("{}/realms/{realm}", base.trim_end_matches('/')))
    }

    /// Bind a concrete Microsoft Entra GUID tenant in the public cloud.
    ///
    /// GUID hexadecimal digits are lowercased. Multi-tenant aliases and domains
    /// are rejected. Sovereign clouds use [`Self::exact_https`] instead.
    pub fn entra_tenant(tenant: &str) -> Result<Self, AuthError> {
        if tenant.len() != 36
            || !tenant.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit()
                }
            })
        {
            return Err(AuthError::InvalidConfiguration);
        }
        Self::exact_https(&format!(
            "https://login.microsoftonline.com/{}/v2.0",
            tenant.to_ascii_lowercase()
        ))
    }

    /// Exact issuer string used by the configured ROM verifier.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Configure ROM verification with host-owned authority, client, and keys.
    ///
    /// The key source must be bound to this issuer and limit time and bytes.
    /// This performs no key fetch or discovery. ROM validates the supplied names.
    pub fn configure<K: TrustedKeys>(
        &self,
        authority: &str,
        client_id: &str,
        source: K,
    ) -> Result<OidcIdTokenAdapter<K>, AuthError> {
        OidcIdTokenAdapter::configured(authority, &self.issuer, client_id, source)
    }
}
