//! Independent public API and native server acceptance outside workspace feature unification.
#[path = "../../../crates/rom-opensearch/tests/support/native_fixture.rs"]
mod native_fixture;
#[path = "../../../crates/rom-opensearch/tests/support/public_case.rs"]
mod public_case;
#[path = "../../../crates/rom-opensearch/tests/support/search_case.rs"]
mod search_case;
#[path = "../../../crates/rom-projection-core/tests/search_fixture/search_permissions.rs"]
mod search_permissions;
#[test]
fn native_writes_and_observations_use_public_api() {
    public_case::run();
}
#[test]
fn sqlite_native_search_uses_public_api() {
    search_case::run(false);
}
#[test]
fn redb_native_search_uses_public_api() {
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
revocations!(sqlite_query_revocation, redb_query_revocation, 1);
revocations!(sqlite_row_revocation, redb_row_revocation, 2);
revocations!(sqlite_field_revocation, redb_field_revocation, 3);

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
