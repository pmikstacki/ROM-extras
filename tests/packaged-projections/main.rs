//! Independent Cargo archive checkpoint consumer; no backend adapter claim.
mod checkpoint_case;
fn main() {
    checkpoint_case::run();
    println!("packaged projection checkpoint close/reopen: passed");
}
