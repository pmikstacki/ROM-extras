use std::fmt;
/// Safe declaration categories without endpoints, credentials or provider replies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmtpError {
    /// Host endpoint, TLS identity or EHLO name is invalid.
    InvalidEndpoint,
    /// Credentials are empty, contain controls or exceed 1024 UTF-8 bytes each.
    InvalidCredentials,
    /// Timeout, reply byte budget or dispatch interval is outside the profile.
    InvalidLimits,
    /// Explicit trust root is empty, invalid or larger than 65536 bytes.
    InvalidCertificate,
    /// TLS configuration could not be constructed.
    ClientInitialization,
}
impl fmt::Display for SmtpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidEndpoint => "invalid SMTP endpoint",
            Self::InvalidCredentials => "invalid SMTP credentials",
            Self::InvalidLimits => "invalid SMTP limits",
            Self::InvalidCertificate => "invalid SMTP certificate",
            Self::ClientInitialization => "SMTP client initialization failed",
        })
    }
}
impl std::error::Error for SmtpError {}
