//! Independent Cargo archive checkpoint consumer; no backend adapter claim.
mod checkpoint_case;
mod native_history_case;
mod worker_case;
fn main() {
    checkpoint_case::run();
    worker_case::run();
    for redb in [false, true] {
        for case in 0..7 {
            native_history_case::run(redb, case);
        }
    }
    println!("packaged projection checkpoint close/reopen: passed");
}
