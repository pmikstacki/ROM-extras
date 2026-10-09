//! Native qualification driver; not a ROM Storage provider or production TLS profile.
use postgres::{Client, Config, NoTls, Transaction, config::Host};
use rom_sql_core::{OwnerError, OwnerState, OwnerToken, OwnerTransaction};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
pub const ID: [u8; 32] = [17; 32];
pub fn connect() -> Client {
    let dsn = std::env::var("ROM_EXTRAS_POSTGRES_DSN")
        .expect("explicit native PostgreSQL fixture required");
    let mut config: Config = dsn.parse().expect("invalid private fixture configuration");
    assert!(!config.get_hosts().is_empty());
    assert!(config.get_hostaddrs().is_empty());
    for host in config.get_hosts() {
        assert!(
            matches!(host, Host::Tcp(host) if host.parse::<std::net::IpAddr>().is_ok_and(|ip|ip.is_loopback()))
        );
    }
    config.connect_timeout(Duration::from_secs(3));
    config.options("-c statement_timeout=3000 -c lock_timeout=1500");
    config
        .connect(NoTls)
        .expect("native fixture connection failed")
}
pub fn provision(client: &mut Client, label: &str) -> String {
    assert!(label.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'));
    let table = format!(
        "extras_owner_{label}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    client.batch_execute(&format!("CREATE TABLE {table} (singleton BOOLEAN PRIMARY KEY CHECK(singleton), format INTEGER NOT NULL, store_id BYTEA NOT NULL CHECK(octet_length(store_id)=32), generation BIGINT NOT NULL CHECK(generation>=0), token BYTEA CHECK(token IS NULL OR octet_length(token)=32), value BIGINT NOT NULL)" )).unwrap();
    client
        .execute(
            &format!("INSERT INTO {table} VALUES (true,1,$1,0,NULL,0)"),
            &[&&ID[..]],
        )
        .unwrap();
    table
}
pub fn token(byte: u8) -> OwnerToken {
    OwnerToken::new([byte; 32]).unwrap()
}
pub struct Tx<'a> {
    pub inner: Transaction<'a>,
    table: String,
    identity: [u8; 32],
    pub lose_ack: bool,
}
impl<'a> Tx<'a> {
    pub fn begin(client: &'a mut Client, table: &str) -> Self {
        Self {
            inner: client.transaction().unwrap(),
            table: table.into(),
            identity: ID,
            lose_ack: false,
        }
    }
    pub fn wrong_identity(mut self) -> Self {
        self.identity = [19; 32];
        self
    }
    pub fn write_value(&mut self, value: i64) -> Result<(), OwnerError> {
        let n = self
            .inner
            .execute(
                &format!("UPDATE {} SET value=$1 WHERE singleton", self.table),
                &[&value],
            )
            .map_err(|_| OwnerError::Unavailable)?;
        if n != 1 {
            return Err(OwnerError::Invalid);
        }
        Ok(())
    }
}
impl OwnerTransaction for Tx<'_> {
    fn lock_owner(&mut self) -> Result<OwnerState, OwnerError> {
        let rows = self
            .inner
            .query(
                &format!(
                    "SELECT format,store_id,generation,token FROM {} WHERE singleton FOR UPDATE",
                    self.table
                ),
                &[],
            )
            .map_err(|e| {
                if e.code() == Some(&postgres::error::SqlState::UNDEFINED_TABLE) {
                    OwnerError::Uninitialized
                } else {
                    OwnerError::Unavailable
                }
            })?;
        let [row] = rows.as_slice() else {
            return Err(OwnerError::Uninitialized);
        };
        if row.get::<_, i32>(0) != 1 {
            return Err(OwnerError::UnsupportedFormat);
        }
        if row.get::<_, Vec<u8>>(1) != self.identity {
            return Err(OwnerError::Invalid);
        }
        let generation = u64::try_from(row.get::<_, i64>(2)).map_err(|_| OwnerError::Invalid)?;
        let token = row
            .get::<_, Option<Vec<u8>>>(3)
            .map(|v| OwnerToken::new(v.try_into().map_err(|_| OwnerError::Invalid)?))
            .transpose()?;
        OwnerState::new(generation, token)
    }
    fn write_owner(&mut self, state: OwnerState) -> Result<(), OwnerError> {
        let bytes = state.token().map(|t| t.as_bytes().to_vec());
        let generation = i64::try_from(state.generation()).map_err(|_| OwnerError::Invalid)?;
        let n=self.inner.execute(&format!("UPDATE {} SET generation=$1,token=$2 WHERE singleton AND format=1 AND store_id=$3",self.table),&[&generation,&bytes,&&self.identity[..]]).map_err(|_|OwnerError::Unavailable)?;
        if n != 1 {
            return Err(OwnerError::Stale);
        }
        Ok(())
    }
    fn commit(self) -> Result<(), OwnerError> {
        self.inner.commit().map_err(|_| OwnerError::Unknown)?;
        if self.lose_ack {
            Err(OwnerError::Unknown)
        } else {
            Ok(())
        }
    }
    fn rollback(self) -> Result<(), OwnerError> {
        self.inner.rollback().map_err(|_| OwnerError::Unknown)
    }
}
pub fn inspect(client: &mut Client, table: &str) -> OwnerState {
    let mut tx = Tx::begin(client, table);
    let state = tx.lock_owner().unwrap();
    tx.rollback().unwrap();
    state
}
pub fn value(client: &mut Client, table: &str) -> i64 {
    client
        .query_one(&format!("SELECT value FROM {table} WHERE singleton"), &[])
        .unwrap()
        .get(0)
}
