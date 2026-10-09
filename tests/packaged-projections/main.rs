//! Independent normalized Cargo archive checkpoint and native OpenSearch consumer.
mod checkpoint_case;
mod native_history_case;
mod public_case;
mod worker_case;
fn main() {
    checkpoint_case::run();
    public_case::run();
    worker_case::run();
    for redb in [false, true] {
        for case in 0..7 {
            native_history_case::run(redb, case);
        }
    }
    println!("packaged projection checkpoint close/reopen: passed");
}
