//! Trusted native provisioning and independently observed exact data.
use mysql::prelude::Queryable;
use rom_mysql::{Config, ControlTable, Deadlines, Profile};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
pub const ID: [u8; 32] = [17; 32];
pub fn profile() -> Profile {
    match std::env::var("ROM_EXTRAS_MYSQL_PROFILE")
        .expect("explicit native profile")
        .as_str()
    {
        "mysql" => Profile::MySql,
        "mariadb" => Profile::MariaDb,
        _ => panic!("invalid native profile"),
    }
}
fn native_options() -> mysql::Opts {
    let key = match profile() {
        Profile::MySql => "ROM_EXTRAS_MYSQL_DSN",
        Profile::MariaDb => "ROM_EXTRAS_MARIADB_DSN",
    };
    let opts = mysql::Opts::from_url(&std::env::var(key).expect("private native DSN")).unwrap();
    let port = match profile() {
        Profile::MySql => 55452,
        Profile::MariaDb => 55453,
    };
    assert!(
        opts.get_ip_or_hostname()
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    );
    assert_eq!(opts.get_tcp_port(), port);
    assert!(opts.get_ssl_opts().is_none());
    opts
}
pub fn config() -> Config {
    let opts = native_options();
    Config::new(
        profile(),
        &opts.get_ip_or_hostname(),
        opts.get_tcp_port(),
        opts.get_user().unwrap(),
        opts.get_pass().unwrap_or_default().into(),
        opts.get_db_name().unwrap(),
    )
    .unwrap()
}
pub fn limits() -> Deadlines {
    Deadlines::new(
        Duration::from_secs(3),
        Duration::from_secs(4),
        Duration::from_secs(1),
    )
    .unwrap()
}
pub fn raw() -> mysql::Conn {
    mysql::Conn::new(
        mysql::OptsBuilder::from_opts(native_options())
            .prefer_socket(false)
            .tcp_connect_timeout(Some(Duration::from_secs(3)))
            .read_timeout(Some(Duration::from_secs(4)))
            .write_timeout(Some(Duration::from_secs(4))),
    )
    .unwrap()
}
pub fn table(label: &str) -> (String, ControlTable) {
    let name = format!(
        "my_{label}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    assert!(name.len() <= 63);
    raw().query_drop(format!("CREATE TABLE `{name}`(singleton_key INT PRIMARY KEY,format_version INT NOT NULL,store_identity LONGBLOB NOT NULL,generation BIGINT NOT NULL,owner_token LONGBLOB,fixture_value BIGINT NOT NULL) ENGINE=InnoDB")).unwrap();
    raw()
        .exec_drop(
            format!("INSERT INTO `{name}` VALUES(1,1,?,0,NULL,0)"),
            (ID.to_vec(),),
        )
        .unwrap();
    let control = ControlTable::new("rom_extras_tests", &name, ID).unwrap();
    (name, control)
}
pub fn update(name: &str) -> String {
    format!("UPDATE `{name}` SET fixture_value=? WHERE singleton_key=1")
}
pub fn value(name: &str) -> i64 {
    raw()
        .query_first(format!(
            "SELECT fixture_value FROM `{name}` WHERE singleton_key=1"
        ))
        .unwrap()
        .unwrap()
}

pub fn connect() -> rom_mysql::Connection {
    rom_mysql::Connection::connect_loopback(config(), limits()).unwrap()
}
pub fn port() -> u16 {
    match profile() {
        Profile::MySql => 55452,
        Profile::MariaDb => 55453,
    }
}
pub fn proxy_config(port: u16) -> Config {
    let o = native_options();
    Config::new(
        profile(),
        "127.0.0.1",
        port,
        o.get_user().unwrap(),
        o.get_pass().unwrap_or_default().into(),
        o.get_db_name().unwrap(),
    )
    .unwrap()
}
pub fn wrong_profile_config(profile: Profile) -> Config {
    let o = native_options();
    Config::new(
        profile,
        &o.get_ip_or_hostname(),
        o.get_tcp_port(),
        o.get_user().unwrap(),
        o.get_pass().unwrap_or_default().into(),
        o.get_db_name().unwrap(),
    )
    .unwrap()
}
pub fn monitor() -> mysql::Conn {
    let password =
        std::env::var("ROM_EXTRAS_MYSQL_MONITOR_PASSWORD").expect("private qualification observer");
    mysql::Conn::new(
        mysql::OptsBuilder::from_opts(native_options())
            .user(Some("root"))
            .pass(Some(password))
            .prefer_socket(false)
            .tcp_connect_timeout(Some(Duration::from_secs(3)))
            .read_timeout(Some(Duration::from_secs(3)))
            .write_timeout(Some(Duration::from_secs(3))),
    )
    .unwrap()
}
