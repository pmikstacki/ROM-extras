//! Actual native search candidates and authoritative public Resource hydration.
#![cfg(feature = "service-fixture")]
#[path = "support/native_fixture.rs"]
mod native_fixture;
#[path = "support/search_case.rs"]
mod search_case;
#[path = "../../rom-projection-core/tests/search_fixture/search_permissions.rs"]
mod search_permissions;
#[test]
fn sqlite_native_typed_search() {
    search_case::run(false);
}
#[test]
fn redb_native_typed_search() {
    search_case::run(true);
}
macro_rules! revocations {
    ($sqlite:ident, $redb:ident, $case:expr) => {
        #[test]
        fn $sqlite() {
            search_case::run_revocation(false, $case);
        }
        #[test]
        fn $redb() {
            search_case::run_revocation(true, $case);
        }
    };
}
revocations!(
    sqlite_native_query_revocation,
    redb_native_query_revocation,
    1
);
revocations!(sqlite_native_row_revocation, redb_native_row_revocation, 2);
revocations!(
    sqlite_native_field_revocation,
    redb_native_field_revocation,
    3
);

#[test]
fn sqlite_native_protected_query_zero_requests() {
    search_case::run_denied(false);
}
#[test]
fn redb_native_protected_query_zero_requests() {
    search_case::run_denied(true);
}

#[test]
fn sqlite_native_held_search_deadline() {
    search_case::run_timeout(false);
}
#[test]
fn redb_native_held_search_deadline() {
    search_case::run_timeout(true);
}

#[test]
fn sqlite_native_mapping_drift_rejects_held_candidates() {
    search_case::run_mapping_drift(false);
}
#[test]
fn redb_native_mapping_drift_rejects_held_candidates() {
    search_case::run_mapping_drift(true);
}
