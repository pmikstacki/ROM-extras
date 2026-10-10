//! Independent maintained CockroachDB ownership, never full ROM Storage.
mod admission;
mod cases;
mod fixture;
mod restart;
#[path = "../../mssql-owner-consumer/src/wire_proxy.rs"]
pub mod wire_proxy;
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 2 && args[0] == "--restart" {
        restart::run(std::path::Path::new(&args[1]));
        return;
    }
    if args.len() == 1 && args[0] == "--malformed" {
        cases::malformed();
        return;
    }
    if args.len() == 1 && args[0] == "--code-safety" {
        cases::malformed_protocol_code();
        return;
    }
    if args.len() == 1 && args[0] == "--ambiguity" {
        cases::ambiguity_rejection();
        return;
    }
    if args.len() == 1 && args[0] == "--locks" {
        cases::ordered_takeover();
        return;
    }
    assert!(args.is_empty(), "unsupported fixture arguments");
    cases::run();
    admission::run();
    println!(
        "Maintained CockroachDB local ownership passed; distributed and full Storage remain unqualified"
    );
}
