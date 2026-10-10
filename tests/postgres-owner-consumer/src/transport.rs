//! Independent native transport budgets and finite SQLSTATE, using public APIs only.
use crate::fixture::config;
use rom_pgwire::{Connection, Error, MetadataLimits};
use rom_sql_core::OperationDeadlines;
use std::time::Duration;

fn connect() -> Connection {
    Connection::connect_loopback(
        config(),
        OperationDeadlines::new(Duration::from_secs(3), Duration::from_secs(4)).unwrap(),
    )
    .unwrap()
}

pub fn run() {
    let mut unexpected = connect();
    assert_eq!(
        unexpected.execute("SELECT 1", &[]),
        Err(Error::ResponseLimit)
    );
    assert!(!unexpected.is_available());
    let mut connection = connect();
    let limits = MetadataLimits::new(2, 1, 8).unwrap();
    let rows = connection
        .query_metadata("SELECT $1::BIGINT", &[&42_i64], limits)
        .unwrap();
    assert_eq!(rows[0].try_get::<_, i64>(0).unwrap(), 42);
    let error = connection.batch("SELECT 1/0").unwrap_err();
    assert_eq!(error.sqlstate(), Some("22012"));
    assert!(connection.is_available());
    assert!(!format!("{error:?}").contains("division"));
    connection.batch("BEGIN; ROLLBACK").unwrap();
    for (sql, limits) in [
        (
            "SELECT generate_series(1,3)",
            MetadataLimits::new(2, 1, 16).unwrap(),
        ),
        ("SELECT 1,2", MetadataLimits::new(1, 1, 16).unwrap()),
        (
            "SELECT '123456789'::TEXT",
            MetadataLimits::new(1, 1, 8).unwrap(),
        ),
    ] {
        let mut connection = connect();
        assert_eq!(
            connection.query_metadata(sql, &[], limits).unwrap_err(),
            Error::ResponseLimit
        );
        assert!(!connection.is_available());
    }
    assert!(MetadataLimits::new(0, 1, 8).is_err());
    assert!(MetadataLimits::new(33, 1, 8).is_err());
    assert!(MetadataLimits::new(1, 17, 8).is_err());
    assert!(MetadataLimits::new(1, 1, 65537).is_err());
    println!("Public PGwire bounded metadata, finite native SQLSTATE and retirement: passed");
}
