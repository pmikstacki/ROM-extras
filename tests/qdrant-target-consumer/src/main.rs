//! Independent host consuming only public provider and ROM contracts.
mod native;
mod recovery;
#[path = "../../../crates/rom-projection-core/tests/search_fixture/search_permissions.rs"]
mod search_permissions;
mod vector;
use native::Host;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let host = Host::load(&args[2]);
    if args[1] == "--vector" {
        vector::run(host, std::path::Path::new(&args[3]), args[4] == "redb").await;
    } else if args[1] == "--definitions" {
        println!("{}", host.definitions());
    } else if args[1] == "--refusal" {
        host.refusal(args[3] == "cancel").await;
    } else if args[1] == "--recovery-write" || args[1] == "--recovery-replay" {
        recovery::run(
            host,
            std::path::Path::new(&args[3]),
            args[4] == "redb",
            args[1] == "--recovery-write",
        )
        .await;
    } else {
        host.run().await;
        println!(
            "Native Qdrant: three metrics, conditional revisions, exact readback and scope passed"
        );
    }
}
