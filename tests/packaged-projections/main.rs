//! Independent normalized Cargo archive checkpoint and native OpenSearch consumer.
mod checkpoint_case;
mod fixture_process;
#[allow(dead_code)]
mod loss_proxy;
mod native_case;
mod native_fixture;
mod native_history_case;
mod process_crash_case;
mod public_case;
mod search_case;
mod search_permissions;
mod worker_case;
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--rom-crash-child") {
        process_crash_case::child();
    }
    checkpoint_case::run();
    public_case::run();
    worker_case::run();
    for redb in [false, true] {
        process_crash_case::run(redb, &["--rom-crash-child"]);
        search_case::run(redb);
        search_case::run_denied(redb);
        search_case::run_timeout(redb);
        search_case::run_mapping_drift(redb);
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
