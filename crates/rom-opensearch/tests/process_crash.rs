//! Process interruption after actual remote acceptance, before checkpoint publication.
#![cfg(feature = "service-fixture")]
#[path = "support/fixture_process.rs"]
mod fixture_process;
#[allow(dead_code)] // Shared fixture also supplies a conversion helper used by other test binaries.
#[path = "support/loss_proxy.rs"]
mod loss_proxy;
#[path = "support/native_fixture.rs"]
mod native_fixture;
#[path = "support/process_crash_case.rs"]
mod process_crash_case;
#[test]
#[ignore = "bounded parent invokes this actual process interruption"]
fn crash_child() {
    process_crash_case::child();
}
#[test]
fn sqlite_process_exit_after_native_acceptance() {
    process_crash_case::run(
        false,
        &["--ignored", "--exact", "crash_child", "--nocapture"],
    );
}
#[test]
fn redb_process_exit_after_native_acceptance() {
    process_crash_case::run(
        true,
        &["--ignored", "--exact", "crash_child", "--nocapture"],
    );
}
