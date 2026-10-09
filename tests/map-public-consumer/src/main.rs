//! Public map-contract consumer outside workspace feature unification.
mod contract;
mod sources;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    contract::run().await;
    sources::run().await;
}
