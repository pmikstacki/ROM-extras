//! Native ownership qualification through maintained public API.
mod cases;
mod fixture;
fn main() {
    cases::run();
    println!("SQL Server native ownership passed; full ROM Storage remains unsupported");
}
