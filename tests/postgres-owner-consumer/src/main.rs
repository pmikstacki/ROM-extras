//! Independent maintained PostgreSQL ownership through public API.
mod admission;
mod cases;
mod fixture;
#[path = "../../mssql-owner-consumer/src/wire_proxy.rs"]
pub mod wire_proxy;
fn main() {
    cases::run();
    admission::run();
    println!("Maintained PostgreSQL ownership passed; full ROM Storage remains unsupported");
}
