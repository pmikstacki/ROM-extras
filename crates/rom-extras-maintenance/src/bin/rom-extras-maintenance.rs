//! Explicit host-only archive inspection and fresh native restoration.
#[path = "../cli.rs"]
mod cli;
fn main() {
    std::process::exit(cli::run());
}
