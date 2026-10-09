//! Public map-contract consumer outside workspace feature unification.
mod browser_grant;
mod contract;
mod sources;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    browser_grant::run();
    contract::run().await;
    sources::run().await;
}
