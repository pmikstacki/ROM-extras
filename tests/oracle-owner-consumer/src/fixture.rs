use rom_oracle::{Config, Connection, ControlTable, Deadlines};
use rom_sql_core::OwnerToken;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
pub(crate) fn bounds() -> Deadlines {
    Deadlines::new(
        Duration::from_secs(10),
        Duration::from_secs(3),
        Duration::from_secs(3),
        Duration::from_secs(1),
    )
    .unwrap()
}
fn credentials() -> (String, String) {
    assert!(
        std::env::var("ROM_EXTRAS_ORACLE_CONNECT").is_ok_and(|v| v == "127.0.0.1:55458/FREEPDB1"),
        "explicit controlled endpoint required"
    );
    let user = std::env::var("ROM_EXTRAS_ORACLE_USER").expect("fixture credential required");
    assert!(user == "ROM_EXTRAS", "least privilege fixture required");
    (
        user,
        std::env::var("ROM_EXTRAS_ORACLE_PASSWORD").expect("fixture credential required"),
    )
}
pub(crate) fn connection() -> Connection {
    let (user, password) = credentials();
    let config = Config::new("127.0.0.1", 55458, "FREEPDB1", &user, &password).unwrap();
    Connection::connect_loopback(config, bounds()).unwrap()
}
pub(crate) fn observer() -> oracle::Connection {
    let (user, password) = credentials();
    let descriptor = "(DESCRIPTION=(CONNECT_TIMEOUT=10sec)(TRANSPORT_CONNECT_TIMEOUT=3sec)(RETRY_COUNT=0)(ADDRESS=(PROTOCOL=TCP)(HOST=127.0.0.1)(PORT=55458))(CONNECT_DATA=(SERVICE_NAME=FREEPDB1)))";
    let c = oracle::Connection::connect(user, password, descriptor)
        .expect("native observer connection");
    c.set_call_timeout(Some(Duration::from_secs(5))).unwrap();
    c
}
pub(crate) fn token(n: u8) -> OwnerToken {
    OwnerToken::new([n; 32]).unwrap()
}
pub(crate) struct Fixture {
    pub(crate) control: ControlTable,
    pub(crate) owner: String,
    pub(crate) data: String,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let name = format!(
            "OWN_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let owner = format!("{name}_C");
        let data = format!("{name}_D");
        let raw = observer();
        raw.execute(&format!("CREATE TABLE {owner} (singleton_key NUMBER PRIMARY KEY,format_version NUMBER,store_identity RAW(2000),generation NUMBER,owner_token RAW(2000))"), &[]).unwrap();
        raw.execute(
            &format!("INSERT INTO {owner} VALUES(1,1,:1,0,NULL)"),
            &[&&[7_u8; 32][..]],
        )
        .unwrap();
        raw.execute(
            &format!("CREATE TABLE {data} (singleton_key NUMBER PRIMARY KEY,value NUMBER)"),
            &[],
        )
        .unwrap();
        raw.execute(&format!("INSERT INTO {data} VALUES(1,0)"), &[])
            .unwrap();
        raw.commit().unwrap();
        let control = ControlTable::new("ROM_EXTRAS", &owner, [7; 32]).unwrap();
        Self {
            control,
            owner,
            data,
        }
    }
    pub(crate) fn update(&self) -> String {
        format!("UPDATE {} SET value=:1 WHERE singleton_key=1", self.data)
    }
    pub(crate) fn value(&self) -> i64 {
        observer()
            .query_row_as(
                &format!("SELECT value FROM {} WHERE singleton_key=1", self.data),
                &[],
            )
            .unwrap()
    }
}
