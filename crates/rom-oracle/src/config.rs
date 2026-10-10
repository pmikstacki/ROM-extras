use rom_sql_core::{ControlTableName, OwnerError};
use std::{
    path::{Component, PathBuf},
    time::Duration,
};
/// Explicit Oracle Net connect/transport, OCI round-trip and native lock bounds.
/// These do not establish a cumulative operation or transaction deadline.
#[derive(Clone, Copy, Debug)]
pub struct Deadlines {
    pub(crate) connect_ms: u64,
    pub(crate) transport_ms: u64,
    pub(crate) roundtrip: Duration,
    pub(crate) lock_seconds: u64,
}
impl Deadlines {
    /// Require positive whole-millisecond bounds through60s, transport<connect and whole-second lock<roundtrip.
    pub fn new(
        connect: Duration,
        transport: Duration,
        roundtrip: Duration,
        lock: Duration,
    ) -> Result<Self, OwnerError> {
        for d in [connect, transport, roundtrip, lock] {
            if d.is_zero()
                || d > Duration::from_secs(60)
                || !d.subsec_nanos().is_multiple_of(1_000_000)
            {
                return Err(OwnerError::Invalid);
            }
        }
        if transport >= connect || lock >= roundtrip || lock.subsec_nanos() != 0 {
            return Err(OwnerError::Invalid);
        }
        Ok(Self {
            connect_ms: connect.as_millis() as u64,
            transport_ms: transport.as_millis() as u64,
            roundtrip,
            lock_seconds: lock.as_secs(),
        })
    }
}
/// Closed endpoint and credentials. Descriptors are generated internally; no native defaults are admitted.
pub struct Config {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) service: String,
    pub(crate) user: String,
    pub(crate) password: String,
}
impl Config {
    /// Validate one literal IP or DNS host, explicit port/service and bounded host-owned credentials.
    pub fn new(
        host: &str,
        port: u16,
        service: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, OwnerError> {
        let host_valid = match host.parse::<std::net::IpAddr>() {
            Ok(ip) => !ip.is_unspecified() && !ip.is_multicast(),
            Err(_) => {
                host.len() <= 253
                    && host.split('.').all(|s| {
                        !s.is_empty()
                            && s.len() <= 63
                            && !s.starts_with('-')
                            && !s.ends_with('-')
                            && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    })
            }
        };
        if !host_valid
            || port == 0
            || service.is_empty()
            || service.len() > 128
            || !service
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
            || user.is_empty()
            || user.len() > 128
            || password.is_empty()
            || password.len() > 1024
            || user.contains('\0')
            || password.contains('\0')
        {
            return Err(OwnerError::Invalid);
        }
        Ok(Self {
            host: host.into(),
            port,
            service: service.into(),
            user: user.into(),
            password: password.into(),
        })
    }
    pub(crate) fn descriptor(&self, d: Deadlines, tls: Option<&TlsWallet>) -> String {
        let security = tls.map(|t| format!("(SECURITY=(SSL_SERVER_DN_MATCH=YES)(SSL_SERVER_CERT_DN=\"{}\")(MY_WALLET_DIRECTORY=\"{}\"))", t.certificate_dn, t.directory.display())).unwrap_or_default();
        let protocol = if tls.is_some() { "TCPS" } else { "TCP" };
        format!(
            "(DESCRIPTION=(CONNECT_TIMEOUT={}ms)(TRANSPORT_CONNECT_TIMEOUT={}ms)(RETRY_COUNT=0)(ADDRESS=(PROTOCOL={protocol})(HOST={})(PORT={}))(CONNECT_DATA=(SERVICE_NAME={})){})",
            d.connect_ms, d.transport_ms, self.host, self.port, self.service, security
        )
    }
}
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Config { redacted }")
    }
}
/// Host-owned native wallet and exact server certificate DN. TCPS qualification remains separate.
pub struct TlsWallet {
    pub(crate) directory: PathBuf,
    pub(crate) certificate_dn: String,
}
impl TlsWallet {
    /// Admit an absolute bounded wallet path and nonempty exact DN without descriptor delimiters or quotes.
    /// The host must protect wallet material and provide native trust files; no verification bypass exists.
    pub fn new(directory: impl Into<PathBuf>, certificate_dn: &str) -> Result<Self, OwnerError> {
        let directory = directory.into();
        let path = directory.to_str().ok_or(OwnerError::Invalid)?;
        let safe = |s: &str| {
            !s.is_empty()
                && s.len() <= 1024
                && !s
                    .chars()
                    .any(|c| c.is_control() || matches!(c, '(' | ')' | '"' | '\\'))
        };
        if !directory.is_absolute()
            || directory
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            || !safe(path)
            || !safe(certificate_dn)
        {
            return Err(OwnerError::Invalid);
        }
        Ok(Self {
            directory,
            certificate_dn: certificate_dn.into(),
        })
    }
}
impl std::fmt::Debug for TlsWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TlsWallet { redacted }")
    }
}
/// Exact quoted native control-table identity. Provision its singleton/index separately.
#[derive(Clone)]
pub struct ControlTable {
    pub(crate) qualified: String,
    pub(crate) identity: [u8; 32],
}
impl ControlTable {
    /// Validate portable schema/table identifiers and preserve case and exact32-byte identity.
    pub fn new(schema: &str, table: &str, identity: [u8; 32]) -> Result<Self, OwnerError> {
        let name = ControlTableName::new(schema, table)?;
        Ok(Self {
            qualified: format!("\"{}\".\"{}\"", name.schema(), name.table()),
            identity,
        })
    }
}
impl std::fmt::Debug for ControlTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ControlTable { redacted }")
    }
}
