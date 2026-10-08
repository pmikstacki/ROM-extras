//! Shared native deadline and panic/rollback assertions.
use postgres::Client;
use rom_sql_core::{Executor, ExecutorError};
use std::{
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const WAIT: Duration = Duration::from_secs(10);

pub(crate) fn table(prefix: &str) -> String {
    format!(
        "extras_{prefix}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

pub(crate) fn deadline(connection: fn() -> Executor<Client>) {
    let executor = connection();
    let name = table("deadline");
    let setup = format!("CREATE TABLE {name} (value BIGINT NOT NULL)");
    executor
        .execute(move |client| client.batch_execute(&setup))
        .unwrap()
        .unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let insert = format!("INSERT INTO {name} VALUES (41)");
    let ticket = executor
        .submit(move |client| {
            let mut tx = client.transaction()?;
            tx.batch_execute(&insert)?;
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(WAIT).unwrap();
            tx.commit()
        })
        .unwrap();
    started_rx.recv_timeout(WAIT).unwrap();
    assert!(matches!(
        ticket.wait_timeout(Duration::ZERO),
        Err(ExecutorError::Unknown)
    ));
    release_tx.send(()).unwrap();
    ticket.wait_timeout(WAIT).unwrap().unwrap();
    executor.shutdown().unwrap();
    let reopened = connection();
    let query = format!("SELECT value FROM {name}");
    let value = reopened
        .execute(move |client| {
            client
                .query_one(&query, &[])
                .map(|row| row.get::<_, i64>(0))
        })
        .unwrap()
        .unwrap();
    assert_eq!(value, 41);
    reopened.shutdown().unwrap();
}

pub(crate) fn panic_rollback(connection: fn() -> Executor<Client>) {
    let executor = connection();
    let name = table("panic");
    let setup = format!("CREATE TABLE {name} (value BIGINT NOT NULL)");
    executor
        .execute(move |client| client.batch_execute(&setup))
        .unwrap()
        .unwrap();
    let insert = format!("INSERT INTO {name} VALUES (9)");
    let failed = executor
        .submit(move |client| {
            let mut tx = client.transaction().unwrap();
            tx.batch_execute(&insert).unwrap();
            panic!("synthetic panic before PGwire commit");
        })
        .unwrap();
    assert!(matches!(
        failed.wait_timeout(WAIT),
        Err(ExecutorError::Unknown)
    ));
    executor.shutdown().unwrap();
    let reopened = connection();
    let query = format!("SELECT count(*) FROM {name}");
    let count = reopened
        .execute(move |client| {
            client
                .query_one(&query, &[])
                .map(|row| row.get::<_, i64>(0))
        })
        .unwrap()
        .unwrap();
    assert_eq!(count, 0);
    reopened.shutdown().unwrap();
}
