//! Public current authorization with controlled candidates and actual SQLite/redb.
#[path = "search_fixture/native_case.rs"]
mod native_case;
#[path = "search_fixture/search_permissions.rs"]
mod search_permissions;
macro_rules! cases {
    ($a:ident, $b:ident, $case:expr) => {
        #[test]
        fn $a() {
            native_case::run(false, $case);
        }
        #[test]
        fn $b() {
            native_case::run(true, $case);
        }
    };
}
cases!(sqlite_denied_zero_calls, redb_denied_zero_calls, 0);
cases!(sqlite_current_hydration, redb_current_hydration, 1);
cases!(sqlite_query_revocation, redb_query_revocation, 2);
cases!(sqlite_row_revocation, redb_row_revocation, 3);
cases!(sqlite_field_revocation, redb_field_revocation, 4);
cases!(sqlite_candidate_overrun, redb_candidate_overrun, 5);
cases!(sqlite_foreign_kind, redb_foreign_kind, 6);
cases!(sqlite_duplicate_candidate, redb_duplicate_candidate, 7);
cases!(sqlite_bounded_output, redb_bounded_output, 8);
cases!(sqlite_closed_source, redb_closed_source, 9);
cases!(
    sqlite_protected_field_zero_calls,
    redb_protected_field_zero_calls,
    10
);

#[test]
fn query_and_candidate_admission_is_bounded_and_debug_is_sanitized() {
    use rom_projection_core::{SearchCandidate, TextMode, TextQuery};
    for (field, text, limit, budget) in [
        ("title", "", 1, 1),
        ("title", " ", 1, 1),
        ("", "word", 1, 1),
        ("title", "word", 0, 1),
        ("title", "word", 65, 65),
        ("title", "word", 2, 1),
        ("title", "word", 1, 257),
    ] {
        assert!(TextQuery::new(field, text, TextMode::AllTerms, limit, budget).is_err());
    }
    assert!(TextQuery::new("title", &"a".repeat(4097), TextMode::AllTerms, 1, 1).is_err());
    assert!(TextQuery::new("title", "a\0b", TextMode::AllTerms, 1, 1).is_err());
    let query = TextQuery::new("title", "private query text", TextMode::AnyTerms, 64, 256).unwrap();
    assert_eq!(format!("{query:?}"), "TextQuery");
    let candidate = SearchCandidate::new(
        rom::Key {
            kind: "docs".into(),
            id: "private-id".into(),
        },
        1,
    )
    .unwrap();
    assert_eq!(format!("{candidate:?}"), "SearchCandidate");
    assert!(SearchCandidate::new(candidate.key().clone(), 0).is_err());
}
