//! Native SQLite/redb public journal acceptance.
#[path = "native_history_fixture/native_history_case.rs"]
mod native_history_case;
#[test]
fn sqlite_tombstone_restart_old_endpoint() {
    native_history_case::run(false, 0);
}
#[test]
fn redb_tombstone_restart_old_endpoint() {
    native_history_case::run(true, 0);
}
#[test]
fn sqlite_revoked_row_blocks_recovery() {
    native_history_case::run(false, 1);
}
#[test]
fn redb_revoked_row_blocks_recovery() {
    native_history_case::run(true, 1);
}
#[test]
fn sqlite_revoked_field_blocks_recovery() {
    native_history_case::run(false, 2);
}
#[test]
fn redb_revoked_field_blocks_recovery() {
    native_history_case::run(true, 2);
}
#[test]
fn sqlite_generation_expiry_closed_source() {
    native_history_case::run(false, 3);
}
#[test]
fn redb_generation_expiry_closed_source() {
    native_history_case::run(true, 3);
}
#[test]
fn sqlite_bounded_selected_vectors() {
    native_history_case::run(false, 4);
}
#[test]
fn redb_bounded_selected_vectors() {
    native_history_case::run(true, 4);
}
#[test]
fn sqlite_denied_empty_page_advances() {
    native_history_case::run(false, 5);
}
#[test]
fn redb_denied_empty_page_advances() {
    native_history_case::run(true, 5);
}
#[test]
fn sqlite_oversize_selection_before_vectors() {
    native_history_case::run(false, 6);
}
#[test]
fn redb_oversize_selection_before_vectors() {
    native_history_case::run(true, 6);
}
