//! Actual public Runtime hydration; candidate transport is explicitly controlled.
#[path = "vector_fixture/case.rs"]
mod case;
#[path = "search_fixture/search_permissions.rs"]
mod search_permissions;
macro_rules! cases {
    ($sqlite:ident,$redb:ident,$n:expr) => {
        #[test]
        fn $sqlite() {
            case::run(false, $n);
        }
        #[test]
        fn $redb() {
            case::run(true, $n);
        }
    };
}
cases!(sqlite_denied_zero_calls, redb_denied_zero_calls, 0);
cases!(
    sqlite_current_authorized_hydration,
    redb_current_authorized_hydration,
    1
);
cases!(
    sqlite_query_revoked_during_dispatch,
    redb_query_revoked_during_dispatch,
    2
);
cases!(sqlite_row_revoked, redb_row_revoked, 3);
cases!(
    sqlite_secondary_embedding_field_hidden,
    redb_secondary_embedding_field_hidden,
    4
);
cases!(
    sqlite_excess_before_closed_source,
    redb_excess_before_closed_source,
    5
);
cases!(
    sqlite_foreign_kind_before_closed_source,
    redb_foreign_kind_before_closed_source,
    6
);
cases!(
    sqlite_duplicate_before_closed_source,
    redb_duplicate_before_closed_source,
    7
);
cases!(sqlite_bounded_native_order, redb_bounded_native_order, 8);
cases!(sqlite_closed_source, redb_closed_source, 9);
cases!(
    sqlite_excluded_before_closed_source,
    redb_excluded_before_closed_source,
    10
);
cases!(sqlite_changed_profile, redb_changed_profile, 11);
cases!(
    sqlite_revocation_after_backend_failure,
    redb_revocation_after_backend_failure,
    12
);
cases!(
    sqlite_foreign_exclusion_zero_calls,
    redb_foreign_exclusion_zero_calls,
    13
);
cases!(
    sqlite_wrong_query_dimension_zero_calls,
    redb_wrong_query_dimension_zero_calls,
    14
);
cases!(sqlite_changed_dimensions, redb_changed_dimensions, 15);
cases!(sqlite_empty_candidates, redb_empty_candidates, 16);
