//! Native ownership qualification through maintained public API.
mod cases;
mod deadlock;
mod fixture;
mod restart;
mod wire;
mod wire_proxy;
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--restart") {
        assert_eq!(args.len(), 2);
        restart::run(std::path::Path::new(&args[1]));
        return;
    }
    assert!(args.is_empty(), "unknown native profile");
    wire::run();
    cases::run();
    deadlock::run();
    println!("SQL Server native ownership passed; full ROM Storage remains unsupported");
}
