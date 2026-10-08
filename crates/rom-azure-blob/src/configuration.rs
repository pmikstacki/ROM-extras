use object_store::{
    ClientOptions, RetryConfig,
    azure::{MicrosoftAzure, MicrosoftAzureBuilder},
};
use rom_blob::{Error, Result};
use std::time::Duration;

/// Explicit host credential; no ambient identity chain is consulted.
pub enum Credentials<'a> {
    /// Base64-encoded account key, kept outside Resource state.
    SharedKey(&'a str),
    /// Host-resolved access token; rebuild the adapter when it rotates.
    BearerToken(&'a str),
}

/// Endpoint policy selected by the host.
#[derive(Clone, Copy)]
pub enum EndpointPolicy {
    /// HTTPS account endpoint with no path, query, fragment or URL credentials.
    HttpsOnly,
    /// Explicit HTTP loopback endpoint whose path is the configured account.
    LoopbackEmulator,
}

/// Host-owned provider configuration. It deliberately does not implement Debug.
pub struct AzureConfig<'a> {
    /// Azure account name.
    pub account: &'a str,
    /// Existing ordinary container name; this adapter does not provision it.
    pub container: &'a str,
    /// Explicit endpoint selected by the host.
    pub endpoint: &'a str,
    /// Endpoint transport policy.
    pub policy: EndpointPolicy,
    /// Explicit host credential.
    pub credentials: Credentials<'a>,
}

/// Bounds for individual adapter operations.
#[derive(Clone, Copy)]
pub struct Limits {
    /// Maximum object size, in 1..=16 MiB.
    pub max_bytes: usize,
    /// Maximum concurrently admitted operations, in 1..=64.
    pub max_in_flight: usize,
    /// Total operation deadline, in 1 ms..=60 seconds.
    pub deadline: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
            max_in_flight: 4,
            deadline: Duration::from_secs(5),
        }
    }
}

pub(crate) fn build(config: AzureConfig<'_>, limits: Limits) -> Result<MicrosoftAzure> {
    if !(1..=16 * 1024 * 1024).contains(&limits.max_bytes)
        || !(1..=64).contains(&limits.max_in_flight)
        || !(Duration::from_millis(1)..=Duration::from_secs(60)).contains(&limits.deadline)
        || !(3..=24).contains(&config.account.len())
        || !config
            .account
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        || !(3..=63).contains(&config.container.len())
        || !config
            .container
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        || config.container.starts_with('-')
        || config.container.ends_with('-')
        || config.container.contains("--")
    {
        return Err(Error::Invalid);
    }
    let url = url::Url::parse(config.endpoint).map_err(|_| Error::Invalid)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host().is_none()
    {
        return Err(Error::Invalid);
    }
    let allow_http = match config.policy {
        EndpointPolicy::HttpsOnly => {
            if url.scheme() != "https" || url.path() != "/" {
                return Err(Error::Invalid);
            }
            false
        }
        EndpointPolicy::LoopbackEmulator => {
            if url.scheme() != "http"
                || url.path() != format!("/{}", config.account)
                || !matches!(url.host(),Some(url::Host::Ipv4(ip)) if ip.is_loopback())
                    && !matches!(url.host(),Some(url::Host::Ipv6(ip)) if ip.is_loopback())
            {
                return Err(Error::Invalid);
            }
            true
        }
    };
    let builder = MicrosoftAzureBuilder::new()
        .with_account(config.account)
        .with_container_name(config.container)
        .with_endpoint(config.endpoint.to_owned())
        .with_use_emulator(false)
        .with_retry(RetryConfig {
            max_retries: 0,
            ..Default::default()
        })
        .with_client_options(
            ClientOptions::new()
                .with_allow_http(allow_http)
                .with_connect_timeout(limits.deadline.min(Duration::from_secs(3)))
                .with_timeout(limits.deadline),
        );
    let builder = match config.credentials {
        Credentials::SharedKey(key) => {
            if key.is_empty() || key.len() > 8192 {
                return Err(Error::Invalid);
            }
            builder
                .with_access_key(key)
                .with_credential_type("access_key")
        }
        Credentials::BearerToken(token) => {
            if token.is_empty()
                || token.len() > 8192
                || !token.bytes().all(|b| (33..=126).contains(&b))
            {
                return Err(Error::Invalid);
            }
            builder
                .with_bearer_token_authorization(token)
                .with_credential_type("bearer_token")
        }
    };
    builder.build().map_err(|_| Error::Invalid)
}
