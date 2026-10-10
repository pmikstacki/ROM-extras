//! Public transport admission and complete startup timeout against an owned blackhole.
use rom_pgwire::{Config, Connection, Error};
use rom_sql_core::OperationDeadlines;
use std::{
    io::Read,
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

fn deadlines() -> OperationDeadlines {
    OperationDeadlines::new(Duration::from_millis(100), Duration::from_secs(1)).unwrap()
}

#[test]
fn no_implicit_endpoint_or_plaintext_remote() {
    for host in [None, Some("localhost"), Some("203.0.113.1")] {
        let mut config = Config::new();
        if let Some(host) = host {
            config.host(host);
        }
        assert_eq!(
            Connection::connect_loopback(config, deadlines()).unwrap_err(),
            Error::Invalid
        );
    }
    let mut config = Config::new();
    config.host("127.0.0.1").hostaddr([127, 0, 0, 1].into());
    assert_eq!(
        Connection::connect_loopback(config, deadlines()).unwrap_err(),
        Error::Invalid
    );
    let mut config = Config::new();
    config.host("127.0.0.1").host("127.0.0.1");
    assert_eq!(
        Connection::connect_loopback(config, deadlines()).unwrap_err(),
        Error::Invalid
    );
}

#[test]
fn startup_timeout_closes_native_socket_before_fixture_cleanup() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let observer = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut bytes = [0; 512];
        loop {
            match socket.read(&mut bytes) {
                Ok(0) => break,
                Ok(_) => (),
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(_) => panic!("startup native socket did not close"),
            }
        }
    });
    let mut config = Config::new();
    config
        .host("127.0.0.1")
        .port(port)
        .user("owned_fixture_probe");
    let started = Instant::now();
    assert_eq!(
        Connection::connect_loopback(config, deadlines()).unwrap_err(),
        Error::Unavailable
    );
    assert!(started.elapsed() < Duration::from_secs(1));
    observer.join().unwrap();
}

#[test]
fn blocking_constructor_refuses_active_host_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut config = Config::new();
        config.host("127.0.0.1");
        assert_eq!(
            Connection::connect_loopback(config, deadlines()).unwrap_err(),
            Error::Invalid
        );
    });
}
