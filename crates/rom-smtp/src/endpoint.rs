use crate::SmtpError;
use rustls::pki_types::{DnsName, ServerName};
use std::net::SocketAddr;
/// One literal host-approved address, separately verified TLS identity and EHLO DNS name.
/// No DNS resolution, URL interpretation, endpoint discovery or proxy is performed.
pub struct Endpoint {
    pub(crate) address: SocketAddr,
    pub(crate) identity: ServerName<'static>,
    pub(crate) hello: String,
}
impl Endpoint {
    /// Validate explicit endpoint settings without connecting.
    pub fn new(
        address: SocketAddr,
        tls_identity: &str,
        hello_name: &str,
    ) -> Result<Self, SmtpError> {
        if address.port() == 0
            || address.ip().is_unspecified()
            || address.ip().is_multicast()
            || tls_identity.len() > 253
            || hello_name.len() > 253
            || !tls_identity.is_ascii()
            || !hello_name.is_ascii()
        {
            return Err(SmtpError::InvalidEndpoint);
        }
        let identity = ServerName::try_from(tls_identity.to_string())
            .map_err(|_| SmtpError::InvalidEndpoint)?;
        DnsName::try_from(hello_name).map_err(|_| SmtpError::InvalidEndpoint)?;
        Ok(Self {
            address,
            identity,
            hello: hello_name.into(),
        })
    }
}
