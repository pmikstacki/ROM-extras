//! Independently compiled public native ownership consumer, with retained tables.
mod cases;
mod driver;
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--race-setup-failure") {
        cases::setup_failure_probe();
        return;
    }
    cases::run();
    println!("Native PostgreSQL ownership transactions passed; not full ROM Storage conformance");
}
