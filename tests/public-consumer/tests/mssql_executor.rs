//! Real SQL Server execution scenarios; these do not establish ROM Storage support.
use rom_sql_core::{Executor, ExecutorError};
use std::{
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tiberius::{AuthMethod, Client, Config, EncryptionLevel};
use tokio::{
    net::TcpStream,
    runtime::{Builder, Runtime},
    time::timeout,
};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

const WAIT: Duration = Duration::from_secs(15);
const IO: Duration = Duration::from_secs(8);

struct Connection {
    client: Client<Compat<TcpStream>>,
    runtime: Runtime,
}
impl Connection {
    fn sql(&mut self, sql: &str) {
        let client = &mut self.client;
        self.runtime.block_on(async {
            timeout(IO, async {
                client.simple_query(sql).await?.into_results().await
            })
            .await
            .expect("SQL fixture deadline")
            .expect("SQL fixture execution");
        });
    }
    fn scalar(&mut self, sql: &str) -> i32 {
        let client = &mut self.client;
        self.runtime.block_on(async {
            timeout(IO, async {
                client.simple_query(sql).await?.into_row().await
            })
            .await
            .expect("SQL fixture deadline")
            .expect("SQL fixture query")
            .expect("SQL fixture row")
            .get(0)
            .expect("SQL fixture integer")
        })
    }
}
fn executor() -> Executor<Connection> {
    let password = std::env::var("ROM_EXTRAS_MSSQL_PASSWORD")
        .expect("Set the required local SQL Server fixture password; never skip this test");
    Executor::spawn(4, move || {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| ExecutorError::Initialization)?;
        let client = runtime.block_on(async {
            timeout(IO, async {
                let mut config = Config::new();
                config.host("127.0.0.1");
                config.port(55440);
                config.database("rom_extras_tests");
                config.authentication(AuthMethod::sql_server("sa", password));
                config.encryption(EncryptionLevel::Required);
                // Only this fixed loopback, synthetic fixture accepts its self-signed certificate.
                config.trust_cert();
                let tcp = TcpStream::connect(config.get_addr()).await?;
                tcp.set_nodelay(true)?;
                Client::connect(config, tcp.compat_write()).await
            })
            .await
            .map_err(|_| ExecutorError::Initialization)?
            .map_err(|_| ExecutorError::Initialization)
        })?;
        let mut connection = Connection { client, runtime };
        connection.sql("SET LOCK_TIMEOUT 5000; SET XACT_ABORT ON");
        Ok(connection)
    })
    .expect("connect to the required SQL Server fixture")
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
async fn async_tds_connection_runs_on_its_worker_inside_a_tokio_host() {
    let connection = executor();
    assert_eq!(
        connection.execute(|c| c.scalar("SELECT CAST(17 AS INT)")),
        Ok(17)
    );
    assert_eq!(connection.execute(|c| c.scalar("SELECT CASE WHEN encrypt_option = 'TRUE' THEN 1 ELSE 0 END FROM sys.dm_exec_connections WHERE session_id = @@SPID")), Ok(1));
    assert_eq!(
        connection.execute(
            |c| c.scalar("SELECT delayed_durability FROM sys.databases WHERE name = DB_NAME()")
        ),
        Ok(0)
    );
    connection.shutdown().unwrap();
}

#[test]
fn caller_timeout_does_not_cancel_a_later_sql_server_commit() {
    let connection = executor();
    let name = table("deadline");
    let ddl = format!("CREATE TABLE {name}(value INT NOT NULL)");
    connection.execute(move |c| c.sql(&ddl)).unwrap();
    let (started, ready) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let insert = format!("BEGIN TRANSACTION; INSERT INTO {name}(value) VALUES(41)");
    let ticket = connection
        .submit(move |c| {
            c.sql(&insert);
            started.send(()).unwrap();
            released.recv_timeout(WAIT).unwrap();
            c.sql("COMMIT TRANSACTION");
        })
        .unwrap();
    ready.recv_timeout(WAIT).unwrap();
    assert_eq!(
        ticket.wait_timeout(Duration::ZERO),
        Err(ExecutorError::Unknown)
    );
    release.send(()).unwrap();
    assert_eq!(ticket.wait_timeout(WAIT), Ok(()));
    connection.shutdown().unwrap();
    let reopened = executor();
    assert_eq!(
        reopened.execute(move |c| c.scalar(&format!("SELECT value FROM {name}"))),
        Ok(41)
    );
    reopened.shutdown().unwrap();
}

#[test]
fn panic_retires_tds_connection_and_open_transaction_rolls_back() {
    let connection = executor();
    let name = table("panic");
    let ddl = format!("CREATE TABLE {name}(value INT NOT NULL)");
    connection.execute(move |c| c.sql(&ddl)).unwrap();
    let insert = format!("BEGIN TRANSACTION; INSERT INTO {name}(value) VALUES(73)");
    let ticket = connection
        .submit(move |c| {
            c.sql(&insert);
            panic!("synthetic TDS job panic before commit");
        })
        .unwrap();
    assert_eq!(ticket.wait_timeout(WAIT), Err(ExecutorError::Unknown));
    connection.shutdown().unwrap();
    let reopened = executor();
    assert_eq!(
        reopened.execute(move |c| c.scalar(&format!("SELECT CAST(COUNT(*) AS INT) FROM {name}"))),
        Ok(0)
    );
    reopened.shutdown().unwrap();
}
