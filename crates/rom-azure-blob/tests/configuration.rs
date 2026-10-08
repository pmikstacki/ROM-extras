//! Refuse unsafe host configuration before constructing a transport.
use rom_azure_blob::{AzureBlob, AzureConfig, Credentials, EndpointPolicy, Limits};
use rom_blob::Error;

fn config(endpoint: &str, policy: EndpointPolicy) -> AzureConfig<'_> {
    AzureConfig {
        account: "fixtureaccount",
        container: "rom-extras",
        endpoint,
        policy,
        credentials: Credentials::BearerToken("synthetic-token"),
    }
}

#[tokio::test]
async fn unsafe_endpoints_are_refused_with_valid_credentials() {
    for endpoint in [
        "http://example.com",
        "https://user:secret@example.com",
        "https://example.com?sig=secret",
        "https://example.com/#fragment",
        "https://example.com/unexpected",
    ] {
        assert!(matches!(
            AzureBlob::connect(
                config(endpoint, EndpointPolicy::HttpsOnly),
                Limits::default()
            ),
            Err(Error::Invalid)
        ));
    }
    for endpoint in [
        "http://example.com/fixtureaccount",
        "http://localhost/fixtureaccount",
        "https://127.0.0.1/fixtureaccount",
        "http://127.0.0.1/otheraccount",
    ] {
        assert!(matches!(
            AzureBlob::connect(
                config(endpoint, EndpointPolicy::LoopbackEmulator),
                Limits::default()
            ),
            Err(Error::Invalid)
        ));
    }
}

#[test]
fn malformed_credentials_and_unbounded_limits_are_rejected() {
    let mut malformed = config(
        "https://fixtureaccount.blob.core.windows.net",
        EndpointPolicy::HttpsOnly,
    );
    malformed.credentials = Credentials::SharedKey("invalid-base64");
    assert!(matches!(
        AzureBlob::connect(malformed, Limits::default()),
        Err(Error::Invalid)
    ));
    for max_bytes in [0, 16 * 1024 * 1024 + 1] {
        let limits = Limits {
            max_bytes,
            ..Limits::default()
        };
        assert!(matches!(
            AzureBlob::connect(
                config(
                    "http://127.0.0.1:55454/fixtureaccount",
                    EndpointPolicy::LoopbackEmulator
                ),
                limits
            ),
            Err(Error::Invalid)
        ));
    }
}

#[tokio::test]
async fn valid_cloud_and_emulator_configuration_construct_clients() {
    for (endpoint, policy) in [
        (
            "https://fixtureaccount.blob.core.windows.net",
            EndpointPolicy::HttpsOnly,
        ),
        (
            "http://127.0.0.1:55454/fixtureaccount",
            EndpointPolicy::LoopbackEmulator,
        ),
    ] {
        assert!(AzureBlob::connect(config(endpoint, policy), Limits::default()).is_ok());
    }
}
