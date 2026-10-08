//! Required, explicitly scoped Azure protocol emulator configuration.
use rom_azure_blob::{AzureBlob, AzureConfig, Credentials, EndpointPolicy, Limits};
pub fn emulator(max_bytes: usize) -> AzureBlob {
    let endpoint =
        std::env::var("ROM_EXTRAS_AZURE_ENDPOINT").expect("Azure emulator endpoint required");
    let account = std::env::var("ROM_EXTRAS_AZURE_ACCOUNT").expect("emulator account required");
    let key = std::env::var("ROM_EXTRAS_AZURE_KEY").expect("private emulator key required");
    let container =
        std::env::var("ROM_EXTRAS_AZURE_CONTAINER").expect("emulator namespace required");
    assert_eq!(endpoint, "http://127.0.0.1:55454/fixtureaccount");
    assert_eq!(account, "fixtureaccount");
    assert_eq!(container, "rom-extras");
    AzureBlob::connect(
        AzureConfig {
            account: &account,
            container: &container,
            endpoint: &endpoint,
            policy: EndpointPolicy::LoopbackEmulator,
            credentials: Credentials::SharedKey(&key),
        },
        Limits {
            max_bytes,
            ..Limits::default()
        },
    )
    .unwrap()
}
