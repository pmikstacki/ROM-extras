//! Independent maintained shared protocol qualification; both vendors run separately.
mod admission;
mod cases;
mod fixture;
mod restart;
#[path = "../../mssql-owner-consumer/src/wire_proxy.rs"]
pub mod wire_proxy;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if matches!(
        args.get(1).map(String::as_str),
        Some("--restart" | "--restart-incomplete")
    ) {
        restart::run(
            std::path::Path::new(args.get(2).expect("owned recovery directory")),
            args[1] == "--restart-incomplete",
        );
        return;
    }
    cases::run();
    admission::run();
    println!("Maintained native ownership passed; full ROM Storage remains unsupported");
}
