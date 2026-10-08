//! Native connection-executor checks; these do not establish ROM Storage support.
use mysql::{Conn, Opts, OptsBuilder, TxOpts, prelude::Queryable};
use rom_sql_core::{Executor, ExecutorError};
use std::{
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const WAIT: Duration = Duration::from_secs(10);

fn options() -> OptsBuilder {
    let profile =
        std::env::var("ROM_EXTRAS_MYSQL_PROFILE").expect("required MySQL/MariaDB profile");
    let variable = match profile.as_str() {
        "mysql" => "ROM_EXTRAS_MYSQL_DSN",
        "mariadb" => "ROM_EXTRAS_MARIADB_DSN",
        _ => panic!("unsupported test profile"),
    };
    let dsn = std::env::var(variable).expect("required database fixture DSN");
    let opts = Opts::from_url(&dsn).expect("invalid fixture configuration");
    assert!(
        opts.get_ip_or_hostname()
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback()),
        "plaintext fixtures require loopback IP"
    );
    assert_eq!(
        opts.get_ip_or_hostname(),
        "127.0.0.1",
        "profile requires its labelled fixture IP"
    );
    assert_eq!(
        opts.get_tcp_port(),
        if profile == "mysql" { 55452 } else { 55453 },
        "profile requires its labelled fixture port"
    );
    assert_eq!(opts.get_db_name(), Some("rom_extras_tests"));
    assert!(
        opts.get_ssl_opts().is_none(),
        "this increment requires the explicit plaintext fixture profile"
    );
    OptsBuilder::from_opts(opts)
        .prefer_socket(false)
        .tcp_connect_timeout(Some(Duration::from_secs(3)))
        .read_timeout(Some(Duration::from_secs(5)))
        .write_timeout(Some(Duration::from_secs(5)))
        .init(vec!["SET SESSION innodb_lock_wait_timeout = 3"])
}

fn connection() -> Executor<Conn> {
    Executor::spawn(4, || {
        Conn::new(options()).map_err(|_| ExecutorError::Initialization)
    })
    .unwrap()
}

fn table(label: &str) -> String {
    format!(
        "extras_{label}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

#[tokio::test(flavor = "current_thread")]
async fn native_client_runs_on_executor_worker_and_checks_durable_profile() {
    let executor = connection();
    let row = executor
        .execute(|c| {
            c.query_first::<(String, u64, u64, u64), _>(
                "SELECT VERSION(), @@innodb_flush_log_at_trx_commit, @@sync_binlog, @@log_bin",
            )
        })
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!((row.1, row.2, row.3), (1, 1, 1));
    match std::env::var("ROM_EXTRAS_MYSQL_PROFILE").unwrap().as_str() {
        "mysql" => assert!(row.0.starts_with("8.4.") && !row.0.contains("MariaDB")),
        "mariadb" => assert!(row.0.starts_with("11.8.") && row.0.contains("MariaDB")),
        _ => unreachable!(),
    }
    println!("Backend: {}", row.0);
    executor.shutdown().unwrap();
}

#[test]
fn waiting_deadline_preserves_unknown_while_native_transaction_later_commits() {
    let executor = connection();
    let name = table("deadline");
    let setup = format!("CREATE TABLE {name} (value BIGINT NOT NULL) ENGINE=InnoDB");
    executor
        .execute(move |c| c.query_drop(setup))
        .unwrap()
        .unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let insert = format!("INSERT INTO {name} VALUES (41)");
    let ticket = executor
        .submit(move |c| {
            let mut tx = c.start_transaction(TxOpts::default())?;
            tx.query_drop(insert)?;
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
    assert_eq!(
        reopened
            .execute(move |c| c.query_first::<i64, _>(query))
            .unwrap()
            .unwrap(),
        Some(41)
    );
    reopened.shutdown().unwrap();
}

#[test]
fn panic_retires_connection_and_uncommitted_rows_are_absent() {
    let executor = connection();
    let name = table("panic");
    let setup = format!("CREATE TABLE {name} (value BIGINT NOT NULL) ENGINE=InnoDB");
    executor
        .execute(move |c| c.query_drop(setup))
        .unwrap()
        .unwrap();
    let insert = format!("INSERT INTO {name} VALUES (9)");
    let ticket = executor
        .submit(move |c| {
            let mut tx = c.start_transaction(TxOpts::default()).unwrap();
            tx.query_drop(insert).unwrap();
            panic!("synthetic panic before native commit");
        })
        .unwrap();
    assert!(matches!(
        ticket.wait_timeout(WAIT),
        Err(ExecutorError::Unknown)
    ));
    assert!(matches!(
        executor.execute(|_| ()),
        Err(ExecutorError::Closed)
    ));
    executor.shutdown().unwrap();
    let reopened = connection();
    let query = format!("SELECT COUNT(*) FROM {name}");
    assert_eq!(
        reopened
            .execute(move |c| c.query_first::<u64, _>(query))
            .unwrap()
            .unwrap(),
        Some(0)
    );
    reopened.shutdown().unwrap();
}

#[test]
fn statement_failure_requires_explicit_whole_operation_rollback() {
    let executor = connection();
    let name = table("rollback");
    let setup = format!(
        "CREATE TABLE {name} (id VARBINARY(32) PRIMARY KEY, value BIGINT NOT NULL) ENGINE=InnoDB"
    );
    executor
        .execute(move |c| c.query_drop(setup))
        .unwrap()
        .unwrap();
    let insert = format!("INSERT INTO {name} VALUES (?, ?)");
    let inside = format!("SELECT COUNT(*) FROM {name}");
    let query = inside.clone();
    executor
        .execute(move |c| {
            let mut tx = c.start_transaction(TxOpts::default()).unwrap();
            tx.exec_drop(&insert, (b"A".as_slice(), 1)).unwrap();
            let err = tx.exec_drop(&insert, (b"A".as_slice(), 2)).unwrap_err();
            assert!(matches!(err,mysql::Error::MySqlError(ref e) if e.code==1062));
            assert_eq!(tx.query_first::<u64, _>(inside).unwrap(), Some(1));
            tx.rollback().unwrap();
        })
        .unwrap();
    assert_eq!(
        executor
            .execute(move |c| c.query_first::<u64, _>(query))
            .unwrap()
            .unwrap(),
        Some(0)
    );
    executor.shutdown().unwrap();
}

#[test]
fn acknowledged_binary_identities_survive_labelled_server_restart() {
    let executor = connection();
    let name = table("restart");
    let setup = format!(
        "CREATE TABLE {name} (id VARBINARY(32) PRIMARY KEY, value BIGINT NOT NULL) ENGINE=InnoDB"
    );
    executor
        .execute(move |c| c.query_drop(setup))
        .unwrap()
        .unwrap();
    let insert = format!("INSERT INTO {name} VALUES (?, ?)");
    executor
        .execute(move |c| {
            let mut tx = c.start_transaction(TxOpts::default()).unwrap();
            for (id, value) in [
                (b"A".as_slice(), 1),
                (b"a".as_slice(), 2),
                ("ż".as_bytes(), 3),
                (b"".as_slice(), 4),
                (b"\0".as_slice(), 5),
            ] {
                tx.exec_drop(&insert, (id, value)).unwrap();
            }
            tx.commit().unwrap();
        })
        .unwrap();
    executor.shutdown().unwrap();
    let profile = std::env::var("ROM_EXTRAS_MYSQL_PROFILE").unwrap();
    let container = format!("rom-extras-{profile}-20261008");
    let label = std::process::Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{ index .Config.Labels \"rom-extras.fixture\" }}",
            &container,
        ])
        .output()
        .unwrap();
    assert!(label.status.success());
    assert_eq!(String::from_utf8(label.stdout).unwrap().trim(), profile);
    assert!(
        std::process::Command::new("docker")
            .args(["restart", "--time", "10", &container])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let reopened = loop {
        match Executor::spawn(4, || {
            Conn::new(options()).map_err(|_| ExecutorError::Initialization)
        }) {
            Ok(executor) => break executor,
            Err(ExecutorError::Initialization) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100))
            }
            Err(_) => panic!("database readiness deadline expired"),
        }
    };
    let query = format!("SELECT id,value FROM {name} ORDER BY value");
    let rows = reopened
        .execute(move |c| c.query::<(Vec<u8>, i64), _>(query))
        .unwrap()
        .unwrap();
    assert_eq!(
        rows,
        vec![
            (b"A".to_vec(), 1),
            (b"a".to_vec(), 2),
            ("ż".as_bytes().to_vec(), 3),
            (vec![], 4),
            (vec![0], 5)
        ]
    );
    reopened.shutdown().unwrap();
}
