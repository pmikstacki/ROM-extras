//! Literal-loopback worker-owned PGwire connections.
use postgres::{Client, Config, NoTls, config::Host};
use rom_sql_core::{Executor, ExecutorError};
use std::time::Duration;

pub(crate) fn connection(dsn_name: &str, statement_timeout_ms: u32) -> Executor<Client> {
    let dsn = std::env::var(dsn_name)
        .expect("required PGwire fixture DSN; missing backend is not a passing test");
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
    assert!(
        config.get_hostaddrs().is_empty(),
        "hostaddr is not allowed for the literal-loopback fixture"
    );
    config.connect_timeout(Duration::from_secs(3));
    config.options(&format!("-c statement_timeout={statement_timeout_ms}"));
    Executor::spawn(4, move || {
        config
            .connect(NoTls)
            .map_err(|_| ExecutorError::Initialization)
    })
    .unwrap()
}
