//! Real CockroachDB execution; not ROM Storage or ownership-fence conformance.
#[path = "../../common/pgwire/mod.rs"]
mod pgwire;
use postgres::Client;
use rom_sql_core::Executor;
use std::{
    sync::{Mutex, mpsc},
    time::Duration,
};
static FIXTURE: Mutex<()> = Mutex::new(());
const WAIT: Duration = Duration::from_secs(10);
fn connection() -> Executor<Client> {
    let dsn = std::env::var("ROM_EXTRAS_COCKROACH_DSN").expect("required Cockroach fixture DSN");
    let config: postgres::Config = dsn.parse().expect("invalid fixture DSN");
    assert!(
        matches!(config.get_hosts(), [postgres::config::Host::Tcp(host)] if host == "127.0.0.1"),
        "fixed Cockroach fixture host required"
    );
    assert_eq!(
        config.get_ports(),
        [55457],
        "fixed Cockroach fixture port required"
    );
    assert_eq!(config.get_dbname(), Some("rom_extras"));
    assert_eq!(config.get_user(), Some("root"));
    assert_eq!(config.get_ssl_mode(), postgres::config::SslMode::Disable);
    pgwire::connection("ROM_EXTRAS_COCKROACH_DSN", 10000)
}

#[tokio::test(flavor = "current_thread")]
async fn typed_worker_inside_tokio_uses_cockroach_serializable_session() {
    let _exclusive = FIXTURE.lock().unwrap();
    let executor = connection();
    let (value, version, isolation) = executor
        .execute(|client| {
            let value = client
                .query_one("SELECT $1::BIGINT", &[&17i64])?
                .get::<_, i64>(0);
            let version = client
                .query_one("SELECT version()", &[])?
                .get::<_, String>(0);
            let isolation = client
                .query_one("SHOW transaction_isolation", &[])?
                .get::<_, String>(0);
            Ok::<_, postgres::Error>((value, version, isolation))
        })
        .unwrap()
        .unwrap();
    assert_eq!(value, 17);
    assert!(version.contains("CockroachDB") && version.contains("v26.3.2"));
    assert_eq!(isolation, "serializable");
    println!("Backend: {version}");
    executor.shutdown().unwrap();
}
#[test]
fn waiting_deadline_does_not_cancel_database_commit() {
    let _exclusive = FIXTURE.lock().unwrap();
    pgwire::deadline(connection);
}
#[test]
fn panicking_job_retires_connection_and_uncommitted_row_is_absent() {
    let _exclusive = FIXTURE.lock().unwrap();
    pgwire::panic_rollback(connection);
}
#[test]
fn duplicate_rejection_rolls_back_preceding_insert_and_fresh_transaction_works() {
    let _exclusive = FIXTURE.lock().unwrap();
    let executor = connection();
    let name = pgwire::cases::table("duplicate");
    executor.execute(move |client| {
        client.batch_execute(&format!("CREATE TABLE {name} (id BIGINT PRIMARY KEY, value BOOL NOT NULL); INSERT INTO {name} VALUES (1, false)"))?;
        let mut tx = client.transaction()?;
        tx.execute(&format!("INSERT INTO {name} VALUES (2, false)"), &[])?;
        let rejection = tx.execute(&format!("INSERT INTO {name} VALUES (1, true)"), &[]).unwrap_err();
        assert_eq!(rejection.code().unwrap().code(), "23505");
        tx.rollback()?;
        assert_eq!(client.query_one(&format!("SELECT count(*) FROM {name} WHERE id=2"), &[])?.get::<_, i64>(0), 0);
        let mut fresh = client.transaction()?;
        fresh.execute(&format!("INSERT INTO {name} VALUES (2, false)"), &[])?;
        fresh.commit()?;
        assert!(!client.query_one(&format!("SELECT value FROM {name} WHERE id=2"), &[])?.get::<_, bool>(0));
        Ok::<_, postgres::Error>(())
    }).unwrap().unwrap();
    executor.shutdown().unwrap();
}
#[test]
fn stale_serializable_read_returns_40001_then_complete_transaction_rereads() {
    let _exclusive = FIXTURE.lock().unwrap();
    let first = connection();
    let second = connection();
    let name = pgwire::cases::table("retry");
    let setup = format!(
        "CREATE TABLE {name} (id BIGINT PRIMARY KEY, value BIGINT NOT NULL); INSERT INTO {name} VALUES (1, 0)"
    );
    first
        .execute(move |client| client.batch_execute(&setup))
        .unwrap()
        .unwrap();
    let (read_tx, read_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let stale_name = name.clone();
    let ticket = first
        .submit(move |client| {
            let mut tx = client.transaction()?;
            let old = tx
                .query_one(&format!("SELECT value FROM {stale_name} WHERE id=1"), &[])?
                .get::<_, i64>(0);
            read_tx.send(old).unwrap();
            release_rx.recv_timeout(WAIT).unwrap();
            let error = tx
                .execute(
                    &format!("UPDATE {stale_name} SET value=$1 WHERE id=1"),
                    &[&(old + 1)],
                )
                .unwrap_err();
            let code = error.code().unwrap().code().to_owned();
            tx.rollback()?;
            Ok::<_, postgres::Error>(code)
        })
        .unwrap();
    assert_eq!(read_rx.recv_timeout(WAIT).unwrap(), 0);
    let change = format!("UPDATE {name} SET value=1 WHERE id=1");
    second
        .execute(move |client| client.batch_execute(&change))
        .unwrap()
        .unwrap();
    release_tx.send(()).unwrap();
    assert_eq!(ticket.wait_timeout(WAIT).unwrap().unwrap(), "40001");
    let query_name = name.clone();
    let fresh_read = first
        .execute(move |client| {
            let mut tx = client.transaction()?;
            let value = tx
                .query_one(&format!("SELECT value FROM {query_name} WHERE id=1"), &[])?
                .get::<_, i64>(0);
            tx.execute(
                &format!("UPDATE {query_name} SET value=$1 WHERE id=1"),
                &[&(value + 1)],
            )?;
            tx.commit()?;
            Ok::<_, postgres::Error>(value)
        })
        .unwrap()
        .unwrap();
    assert_eq!(fresh_read, 1);
    assert_eq!(
        second
            .execute(move |client| client
                .query_one(&format!("SELECT value FROM {name} WHERE id=1"), &[])
                .map(|row| row.get::<_, i64>(0)))
            .unwrap()
            .unwrap(),
        2
    );
    first.shutdown().unwrap();
    second.shutdown().unwrap();
}

#[test]
fn byte_keys_preserve_case_unicode_and_zero_bytes() {
    let _exclusive = FIXTURE.lock().unwrap();
    let executor = connection();
    let name = pgwire::cases::table("identity");
    executor
        .execute(move |client| {
            client.batch_execute(&format!(
                "CREATE TABLE {name} (id BYTEA PRIMARY KEY, value BOOL NOT NULL)"
            ))?;
            let keys: Vec<Vec<u8>> = vec![
                b"A".to_vec(),
                b"a".to_vec(),
                "é".as_bytes().to_vec(),
                "e\u{301}".as_bytes().to_vec(),
                vec![0],
                vec![0, 0],
            ];
            let mut tx = client.transaction()?;
            for key in &keys {
                assert_eq!(
                    tx.execute(
                        &format!("INSERT INTO {name} VALUES ($1, $2)"),
                        &[key, &false]
                    )?,
                    1
                );
            }
            tx.commit()?;
            assert_eq!(
                client
                    .query_one(&format!("SELECT count(*) FROM {name}"), &[])?
                    .get::<_, i64>(0),
                6
            );
            for key in keys {
                let row = client.query_one(
                    &format!("SELECT id, value FROM {name} WHERE id=$1"),
                    &[&key],
                )?;
                assert_eq!(row.get::<_, Vec<u8>>(0), key);
                assert!(!row.get::<_, bool>(1));
            }
            Ok::<_, postgres::Error>(())
        })
        .unwrap()
        .unwrap();
    executor.shutdown().unwrap();
}

#[test]
fn acknowledged_row_survives_restart_of_the_same_disk_store() {
    let _exclusive = FIXTURE.lock().unwrap();
    let executor = connection();
    let name = pgwire::cases::table("retained");
    let setup =
        format!("CREATE TABLE {name} (value BOOL NOT NULL); INSERT INTO {name} VALUES (false)");
    executor
        .execute(move |client| client.batch_execute(&setup))
        .unwrap()
        .unwrap();
    executor.shutdown().unwrap();
    let label = std::process::Command::new("docker")
        .args([
            "inspect",
            "rom-extras-cockroach-20261008",
            "--format",
            "{{index .Config.Labels \"rom-extras.fixture\"}}",
        ])
        .output()
        .unwrap();
    assert!(label.status.success());
    assert_eq!(String::from_utf8(label.stdout).unwrap().trim(), "cockroach");
    assert!(
        std::process::Command::new("timeout")
            .args([
                "315",
                "docker",
                "stop",
                "--time",
                "300",
                "rom-extras-cockroach-20261008"
            ])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    let stopped = std::process::Command::new("docker")
        .args([
            "inspect",
            "rom-extras-cockroach-20261008",
            "--format",
            "{{.State.Status}} {{.State.ExitCode}}",
        ])
        .output()
        .unwrap();
    assert!(stopped.status.success());
    assert_eq!(
        String::from_utf8(stopped.stdout).unwrap().trim(),
        "exited 0",
        "graceful stop must complete without SIGKILL"
    );
    assert!(
        std::process::Command::new("docker")
            .args(["start", "rom-extras-cockroach-20261008"])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let ready = std::process::Command::new("timeout")
            .args([
                "3",
                "docker",
                "exec",
                "rom-extras-cockroach-20261008",
                "/cockroach/cockroach",
                "sql",
                "--insecure",
                "--host=127.0.0.1:26259",
                "--execute=SELECT 1",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        if ready.success() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Cockroach restart readiness deadline"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let reopened = connection();
    let value = reopened
        .execute(move |client| {
            client
                .query_one(&format!("SELECT value FROM {name}"), &[])
                .map(|row| row.get::<_, bool>(0))
        })
        .unwrap()
        .unwrap();
    assert!(!value);
    reopened.shutdown().unwrap();
}
