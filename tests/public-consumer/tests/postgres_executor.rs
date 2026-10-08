//! Real PostgreSQL checks for the connection executor, not ROM Storage conformance.
use postgres::{Client, Config, NoTls, config::Host};
use rom_sql_core::{Executor, ExecutorError};
use std::{
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const WAIT: Duration = Duration::from_secs(10);

fn connection() -> Executor<Client> {
    let dsn = std::env::var("ROM_EXTRAS_POSTGRES_DSN")
        .expect("ROM_EXTRAS_POSTGRES_DSN is required; missing backend is not a passing test");
    let mut config: Config = dsn.parse().expect("invalid test connection configuration");
    assert!(
        !config.get_hosts().is_empty(),
        "explicit loopback host required for NoTls test"
    );
    for host in config.get_hosts() {
        assert!(
            matches!(host, Host::Tcp(host) if host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())),
            "NoTls is restricted to explicit loopback fixture addresses"
        );
    }
    config.connect_timeout(Duration::from_secs(3));
    config.options("-c statement_timeout=3000");
    Executor::spawn(4, move || {
        config
            .connect(NoTls)
            .map_err(|_| ExecutorError::Initialization)
    })
    .unwrap()
}

fn table(prefix: &str) -> String {
    format!(
        "extras_{prefix}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

#[tokio::test(flavor = "current_thread")]
async fn postgres_client_runs_on_its_worker_inside_a_tokio_caller() {
    let executor = connection();
    let value = executor
        .execute(|client| {
            client
                .query_one("SELECT 17::BIGINT", &[])
                .map(|row| row.get::<_, i64>(0))
        })
        .unwrap()
        .unwrap();
    assert_eq!(value, 17);
    let settings = executor
        .execute(|client| {
            client.query_one(
                "SELECT current_setting('fsync'), current_setting('synchronous_commit'), version()",
                &[],
            )
        })
        .unwrap()
        .unwrap();
    assert_eq!(settings.get::<_, String>(0), "on");
    assert_eq!(settings.get::<_, String>(1), "on");
    println!("Backend: {}", settings.get::<_, String>(2));
    executor.shutdown().unwrap();
}

#[test]
fn waiting_deadline_does_not_cancel_the_real_database_commit() {
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

#[test]
fn a_driver_job_panic_retires_its_connection_and_rolls_back_uncommitted_sql() {
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
            panic!("synthetic panic before PostgreSQL commit");
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
