//! Independent public API and native server acceptance outside workspace feature unification.
#[path = "../../../crates/rom-opensearch/tests/support/public_case.rs"]
mod public_case;
#[test]
fn native_writes_and_observations_use_public_api() {
    public_case::run();
}
