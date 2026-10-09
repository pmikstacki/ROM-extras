//! Independent maintained PostgreSQL ownership through public API.
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
    assert!(args.is_empty(), "unsupported fixture arguments");
    cases::run();
    admission::run();
    println!("Maintained PostgreSQL ownership passed; full ROM Storage remains unsupported");
}
