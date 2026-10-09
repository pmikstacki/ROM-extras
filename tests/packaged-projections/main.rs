//! Independent Cargo archive checkpoint consumer; no backend adapter claim.
mod checkpoint_case;
mod worker_case;
fn main() {
    checkpoint_case::run();
    worker_case::run();
    println!("packaged projection checkpoint close/reopen: passed");
}
