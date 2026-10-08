//! Real Oracle executor checks; not ROM Storage conformance.
use oracle::Connection;
use rom_sql_core::{Executor, ExecutorError};
use std::time::Duration;
mod oracle_fixture;
static FIXTURE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn connection() -> Executor<Connection> {
    let user = std::env::var("ROM_EXTRAS_ORACLE_USER").expect("fixture user required");
    let password = std::env::var("ROM_EXTRAS_ORACLE_PASSWORD").expect("fixture password required");
    let address = std::env::var("ROM_EXTRAS_ORACLE_CONNECT").expect("fixture address required");
    assert_eq!(
        address, "127.0.0.1:55458/FREEPDB1",
        "isolated fixture required"
    );
    assert_eq!(user, "ROM_EXTRAS", "least-privilege fixture user required");
    Executor::spawn(4, move || {
        let descriptor = "(DESCRIPTION=(CONNECT_TIMEOUT=10sec)(TRANSPORT_CONNECT_TIMEOUT=3sec)(RETRY_COUNT=0)(ADDRESS=(PROTOCOL=TCP)(HOST=127.0.0.1)(PORT=55458))(CONNECT_DATA=(SERVICE_NAME=FREEPDB1)))";
        let client = Connection::connect(user, password, descriptor)
            .map_err(|_| ExecutorError::Initialization)?;
        client.set_call_timeout(Some(Duration::from_secs(10)))
            .map_err(|_| ExecutorError::Initialization)?;
        Ok(client)
    })
    .expect("Oracle worker initialization")
}

#[tokio::test(flavor = "current_thread")]
async fn oracle_client_runs_on_its_worker_inside_a_tokio_caller() {
    let _fixture = FIXTURE.lock().unwrap();
    let executor = connection();
    let value = executor
        .execute(|client| client.query_row_as::<i64>("SELECT 17 FROM dual", &[]))
        .unwrap()
        .unwrap();
    assert_eq!(value, 17);
    executor.shutdown().unwrap();
}

fn table(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!(
        "EX_{prefix}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

fn create(executor: &Executor<Connection>, name: &str) {
    let sql = format!("CREATE TABLE {name} (value NUMBER(19) NOT NULL)");
    executor
        .execute(move |client| client.execute(&sql, &[]).map(|_| ()))
        .unwrap()
        .unwrap();
}

fn count(name: &str) -> i64 {
    let executor = connection();
    let sql = format!("SELECT count(*) FROM {name}");
    let value = executor
        .execute(move |client| client.query_row_as::<i64>(&sql, &[]))
        .unwrap()
        .unwrap();
    executor.shutdown().unwrap();
    value
}

#[test]
fn waiting_deadline_does_not_cancel_the_real_database_commit() {
    let _fixture = FIXTURE.lock().unwrap();
    use std::sync::mpsc;
    let executor = connection();
    let name = table("DEADLINE");
    create(&executor, &name);
    let sql = format!("INSERT INTO {name} VALUES (41)");
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let ticket = executor
        .submit(move |client| {
            client.execute(&sql, &[])?;
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            client.commit()
        })
        .unwrap();
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(count(&name), 0);
    assert!(matches!(
        ticket.wait_timeout(Duration::ZERO),
        Err(ExecutorError::Unknown)
    ));
    release_tx.send(()).unwrap();
    ticket
        .wait_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    executor.shutdown().unwrap();
    assert_eq!(count(&name), 1);
}

#[test]
fn a_driver_job_panic_retires_its_connection_and_rolls_back_uncommitted_sql() {
    let _fixture = FIXTURE.lock().unwrap();
    let executor = connection();
    let name = table("PANIC");
    create(&executor, &name);
    let sql = format!("INSERT INTO {name} VALUES (9)");
    let ticket = executor
        .submit(move |client| {
            client.execute(&sql, &[]).unwrap();
            panic!("synthetic panic before OCI commit");
        })
        .unwrap();
    assert!(matches!(
        ticket.wait_timeout(Duration::from_secs(10)),
        Err(ExecutorError::Unknown)
    ));
    executor.shutdown().unwrap();
    assert_eq!(count(&name), 0);
}

#[test]
fn unique_violation_keeps_prior_write_pending_until_full_rollback() {
    let _fixture = FIXTURE.lock().unwrap();
    let executor = connection();
    let name = table("UNIQUE");
    let setup = format!("CREATE TABLE {name} (value NUMBER(19) PRIMARY KEY)");
    executor
        .execute(move |client| client.execute(&setup, &[]).map(|_| ()))
        .unwrap()
        .unwrap();
    let insert = format!("INSERT INTO {name} VALUES (:1)");
    executor
        .execute(move |client| -> oracle::Result<()> {
            client.execute(&insert, &[&1_i64])?;
            client.commit()?;
            client.execute(&insert, &[&2_i64])?;
            let error = client.execute(&insert, &[&1_i64]).unwrap_err();
            assert_eq!(error.oci_code(), Some(1));
            // The failed statement does not roll back the preceding statement.
            Ok(())
        })
        .unwrap()
        .unwrap();
    assert_eq!(count(&name), 1);
    let query = format!("SELECT count(*) FROM {name}");
    assert_eq!(
        executor
            .execute(move |client| client.query_row_as::<i64>(&query, &[]))
            .unwrap()
            .unwrap(),
        2
    );
    executor
        .execute(|client| client.rollback())
        .unwrap()
        .unwrap();
    executor.shutdown().unwrap();
    assert_eq!(count(&name), 1);
}

#[test]
fn raw_identifiers_preserve_binary_identity_and_number_zero() {
    let _fixture = FIXTURE.lock().unwrap();
    let executor = connection();
    let name = table("RAW");
    let setup = format!("CREATE TABLE {name} (id RAW(64) PRIMARY KEY, value NUMBER(19) NOT NULL)");
    executor
        .execute(move |client| client.execute(&setup, &[]).map(|_| ()))
        .unwrap()
        .unwrap();
    let insert = format!("INSERT INTO {name} VALUES (:1, :2)");
    executor
        .execute(move |client| -> oracle::Result<()> {
            let keys: Vec<Vec<u8>> = vec![
                b"A".to_vec(),
                b"a".to_vec(),
                "é".as_bytes().to_vec(),
                "e\u{301}".as_bytes().to_vec(),
                vec![0],
                vec![0, 0],
            ];
            for key in keys {
                client.execute(&insert, &[&key, &0_i64])?;
            }
            client.commit()
        })
        .unwrap()
        .unwrap();
    executor.shutdown().unwrap();
    assert_eq!(count(&name), 6);
    let reopened = connection();
    let query = format!("SELECT id, value FROM {name} ORDER BY id");
    let rows = reopened
        .execute(move |client| -> oracle::Result<Vec<(Vec<u8>, i64)>> {
            client.query_as::<(Vec<u8>, i64)>(&query, &[])?.collect()
        })
        .unwrap()
        .unwrap();
    let mut actual: Vec<Vec<u8>> = rows
        .into_iter()
        .map(|(key, value)| {
            assert_eq!(value, 0);
            key
        })
        .collect();
    actual.sort();
    let mut expected = vec![
        b"A".to_vec(),
        b"a".to_vec(),
        "é".as_bytes().to_vec(),
        "e\u{301}".as_bytes().to_vec(),
        vec![0],
        vec![0, 0],
    ];
    expected.sort();
    assert_eq!(actual, expected);
    let empty = reopened
        .execute(|client| client.query_row_as::<Option<String>>("SELECT :1 FROM dual", &[&""]))
        .unwrap()
        .unwrap();
    assert_eq!(empty, None, "Oracle maps an empty VARCHAR string to NULL");
    reopened.shutdown().unwrap();
}

#[test]
fn bounded_admission_rejects_a_job_without_running_it() {
    let _fixture = FIXTURE.lock().unwrap();
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    let executor = connection();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let running = executor
        .submit(move |client| -> oracle::Result<i64> {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            client.query_row_as("SELECT 17 FROM dual", &[])
        })
        .unwrap();
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let queued: Vec<_> = (0..4)
        .map(|_| {
            executor
                .submit(|client| client.query_row_as::<i64>("SELECT 0 FROM dual", &[]))
                .unwrap()
        })
        .collect();
    let rejected_ran = Arc::new(AtomicBool::new(false));
    let observed = rejected_ran.clone();
    assert!(matches!(
        executor.submit(move |_| observed.store(true, Ordering::SeqCst)),
        Err(ExecutorError::Overloaded)
    ));
    release_tx.send(()).unwrap();
    assert_eq!(
        running
            .wait_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap(),
        17
    );
    for ticket in queued {
        assert_eq!(
            ticket
                .wait_timeout(Duration::from_secs(10))
                .unwrap()
                .unwrap(),
            0
        );
    }
    executor.shutdown().unwrap();
    assert!(!rejected_ran.load(Ordering::SeqCst));
}

#[test]
fn acknowledged_row_survives_restart_of_the_same_disk_store() {
    let _fixture = FIXTURE.lock().unwrap();
    let executor = connection();
    let name = table("RESTART");
    create(&executor, &name);
    let sql = format!("INSERT INTO {name} VALUES (0)");
    executor
        .execute(move |client| -> oracle::Result<()> {
            client.execute(&sql, &[])?;
            client.commit()
        })
        .unwrap()
        .unwrap();
    executor.shutdown().unwrap();
    oracle_fixture::restart();
    let reopened = connection();
    let query = format!("SELECT value FROM {name}");
    assert_eq!(
        reopened
            .execute(move |client| client.query_row_as::<i64>(&query, &[]))
            .unwrap()
            .unwrap(),
        0
    );
    reopened.shutdown().unwrap();
}
