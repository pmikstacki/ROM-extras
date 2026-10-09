//! Independent normalized Cargo archive checkpoint and native OpenSearch consumer.
mod checkpoint_case;
mod native_case;
mod native_fixture;
mod native_history_case;
mod public_case;
mod search_case;
mod search_permissions;
mod worker_case;
fn main() {
    checkpoint_case::run();
    public_case::run();
    worker_case::run();
    for redb in [false, true] {
        search_case::run(redb);
        search_case::run_denied(redb);
        for case in 1..=3 {
            search_case::run_revocation(redb, case);
        }
        for case in 0..11 {
            native_case::run(redb, case);
        }
        for case in 0..7 {
            native_history_case::run(redb, case);
        }
    }
    println!("packaged projection checkpoint close/reopen: passed");
}
