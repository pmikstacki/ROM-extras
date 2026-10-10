//! Native schema provisioning and independent observations, never driver policy.
use rom_cockroach::{Config, Connection, ControlTable, Deadlines};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
pub const ID: [u8; 32] = [17; 32];
pub fn config() -> Config {
    let cfg: Config = std::env::var("ROM_EXTRAS_COCKROACH_DSN")
        .expect("explicit fixture")
        .parse()
        .unwrap();
    assert_eq!(cfg.get_ports(), &[55457]);
    assert_eq!(cfg.get_hosts().len(), 1);
    assert!(cfg.get_hostaddrs().is_empty());
    cfg
}
pub fn proxy_config(port: u16) -> Config {
    let original = config();
    let mut cfg = Config::new();
    cfg.host("127.0.0.1")
        .port(port)
        .user(original.get_user().unwrap())
        .dbname(original.get_dbname().unwrap());
    if let Some(password) = original.get_password() {
        cfg.password(password);
    }
    cfg
}
pub fn limits() -> Deadlines {
    Deadlines::new(
        Duration::from_secs(3),
        Duration::from_secs(4),
        Duration::from_secs(3),
        Duration::from_millis(500),
    )
    .unwrap()
}
pub fn connect() -> Connection {
    Connection::connect_loopback(config(), limits()).unwrap()
}
pub fn raw() -> postgres::Client {
    let mut cfg: postgres::Config = std::env::var("ROM_EXTRAS_COCKROACH_DSN")
        .unwrap()
        .parse()
        .unwrap();
    assert!(cfg.get_hostaddrs().is_empty());
    assert_eq!(cfg.get_hosts().len(), 1);
    assert!(
        matches!(&cfg.get_hosts()[0],postgres::config::Host::Tcp(host) if host.parse::<std::net::IpAddr>().is_ok_and(|ip|ip.is_loopback()))
    );
    cfg.connect_timeout(Duration::from_secs(3));

    let mut connection = cfg.connect(postgres::NoTls).unwrap();
    connection
        .batch_execute("SET statement_timeout='4000ms'; SET lock_timeout='3000ms'")
        .unwrap();
    connection
}
pub fn table(label: &str) -> (String, ControlTable) {
    let name = format!(
        "cr_{label}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    assert!(name.len() <= 63);
    raw().batch_execute(&format!("CREATE TABLE public.{name}(singleton_key INT4 PRIMARY KEY CHECK(singleton_key=1),format_version INT4 NOT NULL,store_identity BYTEA NOT NULL,generation BIGINT NOT NULL,owner_token BYTEA,fixture_value BIGINT NOT NULL); INSERT INTO public.{name} VALUES(1,1,decode('{}','hex'),0,NULL,0)","11".repeat(32))).unwrap();
    let control = ControlTable::new("public", &name, ID).unwrap();
    (name, control)
}
pub fn value(name: &str) -> i64 {
    raw()
        .query_one(
            &format!("SELECT fixture_value FROM public.{name} WHERE singleton_key=1"),
            &[],
        )
        .unwrap()
        .get(0)
}
pub fn update(name: &str) -> String {
    format!("UPDATE public.{name} SET fixture_value=$1 WHERE singleton_key=1")
}
