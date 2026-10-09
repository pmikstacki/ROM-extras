//! Fixture provisioning is separate from maintained driver operations.
use rom_mssql::{Connection, ControlTable, Deadlines};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tiberius::{AuthMethod, Client, Config, EncryptionLevel};
use tokio::{net::TcpStream, runtime::Builder};
use tokio_util::compat::TokioAsyncWriteCompatExt;

pub const WAIT: Duration = Duration::from_secs(8);
pub const ID: [u8; 32] = [17; 32];
pub fn config() -> Config {
    let mut config = Config::new();
    config.host("127.0.0.1");
    config.port(55440);
    config.database("rom_extras_tests");
    config.authentication(AuthMethod::sql_server(
        "sa",
        std::env::var("ROM_EXTRAS_MSSQL_PASSWORD").expect("explicit fixture password"),
    ));
    config.encryption(EncryptionLevel::Required);
    config.trust_cert(); // Synthetic, fixed loopback only. The maintained adapter never calls this.
    config
}
pub fn connect() -> Connection {
    Connection::connect(
        config(),
        Deadlines::new(WAIT, Duration::from_secs(4), Duration::from_millis(500)).unwrap(),
    )
    .unwrap()
}
pub struct FixtureSession {
    client: Client<tokio_util::compat::Compat<TcpStream>>,
    rt: tokio::runtime::Runtime,
}
impl FixtureSession {
    pub fn new() -> Self {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let client = rt.block_on(async {
            tokio::time::timeout(WAIT, async {
                let cfg = config();
                let tcp = TcpStream::connect(cfg.get_addr()).await.unwrap();
                tcp.set_nodelay(true).unwrap();
                Client::connect(cfg, tcp.compat_write()).await.unwrap()
            })
            .await
            .expect("fixture connect deadline")
        });
        Self { client, rt }
    }
    pub fn sql(&mut self, sql: &str) -> Vec<Vec<tiberius::Row>> {
        self.rt.block_on(async {
            tokio::time::timeout(Duration::from_secs(20), async {
                self.client
                    .simple_query(sql)
                    .await
                    .unwrap()
                    .into_results()
                    .await
                    .unwrap()
            })
            .await
            .expect("fixture native statement deadline")
        })
    }
}
pub fn sql(sql: &str) -> Vec<Vec<tiberius::Row>> {
    FixtureSession::new().sql(sql)
}
pub fn scalar(sql_text: &str) -> i64 {
    sql(sql_text)[0][0].get(0).unwrap()
}
pub fn table(label: &str) -> (String, ControlTable) {
    let name = format!(
        "mssql_{label}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    assert!(name.len() <= 63);
    sql(&format!(
        "CREATE TABLE dbo.[{name}] (singleton_key INT NOT NULL PRIMARY KEY CHECK(singleton_key=1), format_version INT NOT NULL, store_identity VARBINARY(MAX) NOT NULL, generation BIGINT NOT NULL, owner_token VARBINARY(MAX) NULL, fixture_value BIGINT NOT NULL); INSERT dbo.[{name}] VALUES(1,1,0x{},0,NULL,0)",
        "11".repeat(32)
    ));
    let control = ControlTable::new("dbo", &name, ID).unwrap();
    (name, control)
}
pub fn value(name: &str) -> i64 {
    scalar(&format!("SELECT fixture_value FROM dbo.[{name}]"))
}
pub fn update(name: &str) -> String {
    format!("UPDATE dbo.[{name}] SET fixture_value=@P1 WHERE singleton_key=1")
}
