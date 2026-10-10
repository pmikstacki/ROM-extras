//! Public limits and real protocol-negative admission, without fake successful services.
use crate::fixture::*;
use rom_cockroach::{Connection, ControlTable, Deadlines};
use rom_sql_core::{OwnerError, OwnerToken, claim_owner, with_owner};
use std::{
    net::TcpListener,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
pub fn run() {
    for text in ["", "0bad", "bad\"name", "bad;sql", &"x".repeat(64)] {
        assert!(ControlTable::new("public", text, ID).is_err());
    }
    assert!(
        Deadlines::new(
            Duration::from_secs(61),
            Duration::from_secs(4),
            Duration::from_secs(3),
            Duration::from_millis(500)
        )
        .is_err()
    );
    assert!(
        Deadlines::new(
            Duration::from_secs(3),
            Duration::from_secs(3),
            Duration::from_secs(3),
            Duration::from_millis(500)
        )
        .is_err()
    );
    assert!(
        Deadlines::new(
            Duration::from_secs(3),
            Duration::from_secs(4),
            Duration::from_secs(3),
            Duration::from_nanos(1)
        )
        .is_err()
    );
    let mut cfg = config();
    cfg.host("127.0.0.1");
    assert_eq!(
        Connection::connect_loopback(cfg, limits()).unwrap_err(),
        OwnerError::Invalid
    );
    let mut cfg = config();
    cfg.hostaddr([127, 0, 0, 1].into());
    assert_eq!(
        Connection::connect_loopback(cfg, limits()).unwrap_err(),
        OwnerError::Invalid
    );
    let mut cfg = rom_cockroach::Config::new();
    cfg.host("203.0.113.1");
    assert_eq!(
        Connection::connect_loopback(cfg, limits()).unwrap_err(),
        OwnerError::Invalid
    );
    // Native server or the supplied NoTls connector must reject required encryption.
    assert_eq!(
        Connection::connect(config(), postgres::NoTls, limits()).unwrap_err(),
        OwnerError::Unavailable
    );
    let wrong: rom_cockroach::Config = std::env::var("ROM_EXTRAS_POSTGRES_DSN")
        .expect("explicit PostgreSQL negative fixture")
        .parse()
        .unwrap();
    assert_eq!(wrong.get_ports(), &[55439]);
    assert_eq!(
        Connection::connect_loopback(wrong, limits()).unwrap_err(),
        OwnerError::Unavailable
    );
    let (name, control) = table("limit");
    let mut c = connect();
    let owner = claim_owner(
        c.begin(&control).unwrap(),
        OwnerToken::new([11; 32]).unwrap(),
    )
    .unwrap();
    let parameters = vec![&1_i64 as &(dyn rom_cockroach::ToSql + Sync); 65];
    assert_eq!(
        with_owner(c.begin(&control).unwrap(), owner, |tx| {
            tx.execute(&update(&name), &[&99_i64])?;
            tx.execute(&update(&name), &parameters).map(|_| ())
        }),
        Err(OwnerError::Invalid)
    );
    assert_eq!(value(&name), 0);
    assert_eq!(
        with_owner(c.begin(&control).unwrap(), owner, |tx| tx
            .execute(&"x".repeat(16385), &[])
            .map(|_| ())),
        Err(OwnerError::Invalid)
    );
    assert_eq!(value(&name), 0);
    connect_blackhole();
    statement_timeout();
    println!("Native endpoint/TLS denial, byte/parameter bounds and connect deadline: passed");
}
fn connect_blackhole() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let (stop, stopped) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let end = Instant::now() + Duration::from_secs(2);
        let socket = loop {
            assert!(Instant::now() < end);
            match listener.accept() {
                Ok((socket, _)) => break socket,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2))
                }
                Err(_) => panic!("owned blackhole accept"),
            }
        };
        stopped.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(socket);
    });
    let mut cfg = rom_cockroach::Config::new();
    cfg.host("127.0.0.1")
        .port(port)
        .user("public_fixture_probe");
    let start = Instant::now();
    assert_eq!(
        Connection::connect_loopback(
            cfg,
            Deadlines::new(
                Duration::from_millis(100),
                Duration::from_secs(2),
                Duration::from_secs(1),
                Duration::from_millis(500)
            )
            .unwrap()
        )
        .unwrap_err(),
        OwnerError::Unavailable
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    stop.send(()).unwrap();
    worker.join().unwrap();
}
fn statement_timeout() {
    let (name, control) = table("stmt");
    let mut c = Connection::connect_loopback(
        config(),
        Deadlines::new(
            Duration::from_secs(3),
            Duration::from_secs(2),
            Duration::from_secs(1),
            Duration::from_millis(500),
        )
        .unwrap(),
    )
    .unwrap();
    let owner = claim_owner(
        c.begin(&control).unwrap(),
        OwnerToken::new([12; 32]).unwrap(),
    )
    .unwrap();
    let start = Instant::now();
    assert_eq!(
        with_owner(c.begin(&control).unwrap(), owner, |tx| {
            tx.execute(&update(&name), &[&99_i64])?;
            tx.execute("SELECT pg_sleep(3)", &[]).map(|_| ())
        }),
        Err(OwnerError::Unavailable)
    );
    assert!(
        start.elapsed() >= Duration::from_millis(900) && start.elapsed() < Duration::from_secs(2)
    );
    assert!(c.is_available());
    assert_eq!(value(&name), 0);
    with_owner(c.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&42_i64]).map(|_| ())
    })
    .unwrap();
    assert_eq!(value(&name), 42);
    println!("Native statement timeout prefix rollback and explicit reuse: passed");
}
