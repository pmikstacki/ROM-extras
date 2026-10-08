use crate::WebhookError;
use std::net::{IpAddr, SocketAddr};
use url::{Host, Url};

/// Host-configured HTTPS destination with a fixed, validated address set.
///
/// No Debug/serialization API exposes query credentials. Addresses are pinned
/// for the transport lifetime; rebuild the transport when host resolution changes.
pub struct Destination {
    pub(crate) url: Url,
    pub(crate) addresses: Vec<SocketAddr>,
}
impl Destination {
    /// Bind a host-resolved address set without a second DNS lookup at connection.
    ///
    /// All 1..=64 addresses must pass the conservative public-address profile.
    /// IP literal URLs require exactly their own address. TLS verifies the URL
    /// hostname, not a replacement IP. The host supplies DNS resolution.
    pub fn public_resolved(raw: &str, addresses: &[IpAddr]) -> Result<Self, WebhookError> {
        Self::validated(raw, addresses, is_public)
    }

    /// Bind only loopback IPs for an explicitly enabled local TLS fixture.
    /// This never disables TLS hostname or certificate verification.
    #[cfg(feature = "loopback-fixture")]
    pub fn loopback_fixture(raw: &str, addresses: &[IpAddr]) -> Result<Self, WebhookError> {
        Self::validated(raw, addresses, IpAddr::is_loopback)
    }

    fn validated(
        raw: &str,
        addresses: &[IpAddr],
        allowed: impl Fn(&IpAddr) -> bool,
    ) -> Result<Self, WebhookError> {
        if raw.len() > 2048
            || !raw.starts_with("https://")
            || raw
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
        {
            return Err(WebhookError::InvalidDestination);
        }
        let url = Url::parse(raw).map_err(|_| WebhookError::InvalidDestination)?;
        let authority = raw[8..].split(['/', '?', '#']).next().unwrap_or_default();
        if authority.is_empty()
            || authority.contains('@')
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || url.host().is_none()
        {
            return Err(WebhookError::InvalidDestination);
        }
        if addresses.is_empty() || addresses.len() > 64 || addresses.iter().any(|ip| !allowed(ip)) {
            return Err(WebhookError::ForbiddenAddress);
        }
        let literal = match url.host().expect("validated host") {
            Host::Ipv4(ip) => Some(IpAddr::V4(ip)),
            Host::Ipv6(ip) => Some(IpAddr::V6(ip)),
            Host::Domain(_) => None,
        };
        if let Some(ip) = literal
            && (addresses.len() != 1 || addresses[0] != ip)
        {
            return Err(WebhookError::ForbiddenAddress);
        }
        let port = url
            .port_or_known_default()
            .ok_or(WebhookError::InvalidDestination)?;
        if port == 0 {
            return Err(WebhookError::InvalidDestination);
        }
        Ok(Self {
            url,
            addresses: addresses
                .iter()
                .map(|ip| SocketAddr::new(*ip, port))
                .collect(),
        })
    }
}

// Conservative snapshot of IANA special-purpose allocations, 2025-10-09.
// Reject even globally reachable special allocations. IPv6 outside 2000::/3
// (including all mapped/translation/link-local/private/multicast) is excluded.
fn is_public(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let n = u32::from(*ip);
            let denied = [
                (0x00000000, 8),
                (0x0a000000, 8),
                (0x64400000, 10),
                (0x7f000000, 8),
                (0xa9fe0000, 16),
                (0xac100000, 12),
                (0xc0000000, 24),
                (0xc0000200, 24),
                (0xc01fc400, 24),
                (0xc034c100, 24),
                (0xc0586300, 24),
                (0xc0a80000, 16),
                (0xc0af3000, 24),
                (0xc6120000, 15),
                (0xc6336400, 24),
                (0xcb007100, 24),
                (0xe0000000, 4),
                (0xf0000000, 4),
            ];
            !denied
                .iter()
                .any(|(prefix, bits)| n >> (32 - bits) == prefix >> (32 - bits))
        }
        IpAddr::V6(ip) => {
            let n = u128::from(*ip);
            if n >> 125 != 1 {
                return false;
            }
            let denied: [(u128, u32); 5] = [
                (0x2001_u128 << 112, 23),
                (0x20010db8_u128 << 96, 32),
                (0x2002_u128 << 112, 16),
                (0x2620004f8000_u128 << 80, 48),
                (0x3fff_u128 << 112, 20),
            ];
            !denied
                .iter()
                .any(|(prefix, bits)| n >> (128 - bits) == prefix >> (128 - bits))
        }
    }
}
