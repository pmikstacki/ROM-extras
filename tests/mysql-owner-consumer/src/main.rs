//! Independent maintained shared protocol qualification; both vendors run separately.
mod admission;
mod cases;
mod fixture;
#[path = "../../mssql-owner-consumer/src/wire_proxy.rs"]
pub mod wire_proxy;
fn main() {
    cases::run();
    admission::run();
    println!("Maintained native ownership passed; full ROM Storage remains unsupported");
}
