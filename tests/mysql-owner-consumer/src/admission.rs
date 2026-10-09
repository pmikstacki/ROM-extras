use crate::fixture::*;
use rom_mysql::{Config, Connection, ControlTable, Deadlines, Profile, SslOpts, Value};
use rom_sql_core::{OwnerError, OwnerToken, OwnerTransaction, claim_owner, with_owner};
use std::{
    io::Read,
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};
pub fn run() {
    for s in ["", "0bad", "bad`sql", "bad;sql", &"x".repeat(64)] {
        assert!(ControlTable::new("rom_extras_tests", s, ID).is_err());
    }
    for s in ["", "bad/path", "bad@host", "bad host"] {
        assert!(
            Config::new(
                Profile::MySql,
                s,
                55452,
                "fixture",
                "synthetic_secret".into(),
                "rom_extras_tests"
            )
            .is_err()
        );
    }
    for d in [
        Duration::ZERO,
        Duration::from_millis(500),
        Duration::from_secs(4),
    ] {
        assert!(Deadlines::new(Duration::from_secs(3), Duration::from_secs(4), d).is_err());
    }
    assert!(
        Deadlines::new(
            Duration::from_secs(61),
            Duration::from_secs(4),
            Duration::from_secs(1)
        )
        .is_err()
    );
    assert!(
        !format!(
            "{:?}",
            Config::new(
                profile(),
                "127.0.0.1",
                port(),
                "private_user",
                "synthetic_secret".into(),
                "rom_extras_tests"
            )
            .unwrap()
        )
        .contains("synthetic_secret")
    );
    assert_eq!(
        Connection::connect(
            config(),
            SslOpts::default().with_danger_accept_invalid_certs(true),
            limits()
        )
        .unwrap_err(),
        OwnerError::Invalid
    );
    let wrong = match profile() {
        Profile::MySql => Profile::MariaDb,
        Profile::MariaDb => Profile::MySql,
    };
    // Use existing private credentials while changing only the explicit vendor expectation.
    let cfg = super::fixture::wrong_profile_config(wrong);
    assert_eq!(
        Connection::connect_loopback(cfg, limits()).unwrap_err(),
        OwnerError::Unavailable
    );
    let (name, ctl) = table("limit");
    let mut c = connect();
    let owner = claim_owner(c.begin(&ctl).unwrap(), OwnerToken::new([11; 32]).unwrap()).unwrap();
    for (sql, params) in [
        (update(&name), vec![Value::Int(1); 65]),
        ("x".repeat(16385), vec![]),
        (update(&name), vec![Value::Bytes(vec![1; 61440])]),
    ] {
        assert_eq!(
            with_owner(c.begin(&ctl).unwrap(), owner, |tx| {
                tx.execute(&update(&name), vec![99_i64.into()])?;
                let _ = tx.execute(&sql, params);
                Ok(())
            }),
            Err(OwnerError::Invalid)
        );
        assert_eq!(value(&name), 0);
    }
    let mut tx = c.begin(&ctl).unwrap();
    tx.lock_owner().unwrap();
    assert_eq!(tx.execute("SELECT 1", vec![]), Err(OwnerError::Unknown));
    assert_eq!(tx.rollback(), Err(OwnerError::Unknown));
    assert!(!c.is_available());
    blackhole();
    statement_timeout();
    println!(
        "Native vendor/config/TLS denial, parameter bounds, unexpected rowset retirement and connect deadline: passed"
    );
}
fn blackhole() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let p = listener.local_addr().unwrap().port();
    let worker = thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut b = [0u8; 16];
        assert_eq!(
            s.read(&mut b).unwrap(),
            0,
            "constructor must close native peer before fixture cleanup"
        );
    });
    let start = Instant::now();
    assert_eq!(
        Connection::connect_loopback(
            proxy_config(p),
            Deadlines::new(
                Duration::from_millis(150),
                Duration::from_secs(2),
                Duration::from_secs(1)
            )
            .unwrap()
        )
        .unwrap_err(),
        OwnerError::Unavailable
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    worker.join().unwrap();
}
fn statement_timeout() {
    let (_, ctl) = table("timeout");
    let mut c = Connection::connect_loopback(
        config(),
        Deadlines::new(
            Duration::from_secs(3),
            Duration::from_secs(2),
            Duration::from_secs(1),
        )
        .unwrap(),
    )
    .unwrap();
    let mut tx = c.begin(&ctl).unwrap();
    tx.lock_owner().unwrap();
    let start = Instant::now();
    assert_eq!(tx.execute("DO SLEEP(5)", vec![]), Err(OwnerError::Unknown));
    assert!(start.elapsed() < Duration::from_secs(3));
    assert_eq!(tx.rollback(), Err(OwnerError::Unknown));
    assert!(!c.is_available());
}
