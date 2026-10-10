//! Retained guards across actual native process death; not host power-loss evidence.
use crate::fixture::*;
use mysql::prelude::Queryable;
use rom_mysql::Connection;
use rom_sql_core::{
    OwnerError, OwnerToken, OwnerTransaction, claim_owner, release_owner, takeover_owner,
    with_owner,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    thread,
    time::{Duration, Instant},
};

fn native_profile() -> (String, u64) {
    let mut observer = raw();
    let row: (String, u64, u64, u64, u64) = observer.query_first("SELECT VERSION(),@@GLOBAL.innodb_flush_log_at_trx_commit,@@GLOBAL.log_bin,@@GLOBAL.sync_binlog,@@GLOBAL.innodb_force_recovery").unwrap().unwrap();
    assert_eq!(row.1, 1);
    if row.2 == 1 {
        assert_eq!(row.3, 1);
    }
    assert_eq!(row.4, 0);
    let status: (String, String) = observer
        .query_first("SHOW GLOBAL STATUS LIKE 'Uptime'")
        .unwrap()
        .unwrap();
    (row.0, status.1.parse().unwrap())
}

pub fn run(directory: &Path, incomplete: bool) {
    assert!(directory.is_dir());
    let deadline = Instant::now() + Duration::from_secs(8);
    let before = loop {
        let p = native_profile();
        if p.1 >= 5 {
            break p;
        }
        assert!(
            Instant::now() < deadline,
            "native uptime admission deadline"
        );
        thread::sleep(Duration::from_millis(100));
    };
    let (name, control) = table("crash");
    let (probe, _) = table("undo");
    let mut c = connect();
    let old = claim_owner(
        c.begin(&control).unwrap(),
        OwnerToken::new([61; 32]).unwrap(),
    )
    .unwrap();
    with_owner(c.begin(&control).unwrap(), old, |tx| {
        tx.execute(&update(&name), vec![17_i64.into()]).map(|_| ())
    })
    .unwrap();
    let owner = takeover_owner(
        c.begin(&control).unwrap(),
        old.state(),
        OwnerToken::new([62; 32]).unwrap(),
    )
    .unwrap();
    assert_eq!(owner.generation(), 2);
    assert_eq!(value(&name), 17);
    drop(c); // Both acknowledged guards remain in RAM; no release or reconstruction.
    let held = if incomplete {
        let mut tx = raw();
        tx.query_drop("START TRANSACTION").unwrap();
        tx.exec_drop(update(&probe), (99_i64,)).unwrap();
        let own: i64 = tx
            .query_first(format!(
                "SELECT fixture_value FROM `{probe}` WHERE singleton_key=1"
            ))
            .unwrap()
            .unwrap();
        assert_eq!(own, 99);
        assert_eq!(value(&probe), 0); // Independent committed reader; transaction stays open through SIGKILL.
        Some(tx)
    } else {
        None
    };
    let mut signal = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.join("prepared.tmp"))
        .unwrap();
    signal.write_all(b"ready\n").unwrap();
    signal.sync_all().unwrap();
    drop(signal);
    fs::rename(directory.join("prepared.tmp"), directory.join("prepared")).unwrap();
    let end = Instant::now() + Duration::from_secs(40);
    while !directory.join("restarted").exists() {
        assert!(Instant::now() < end, "owned fixture restart deadline");
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        fs::read_to_string(directory.join("restarted")).unwrap(),
        "resume\n"
    );
    drop(held); // Native transaction was not closed before process death.
    let end = Instant::now() + Duration::from_secs(30);
    let mut c = loop {
        match Connection::connect_loopback(config(), limits()) {
            Ok(c) => break c,
            Err(_) => {
                assert!(Instant::now() < end, "native readiness deadline");
                thread::sleep(Duration::from_millis(50));
            }
        }
    };
    let after = native_profile();
    assert_eq!(before.0, after.0, "native version changed");
    assert!(after.1 < before.1, "native uptime did not reset");
    let mut tx = c.begin(&control).unwrap();
    assert_eq!(tx.lock_owner().unwrap(), owner.state());
    tx.rollback().unwrap();
    assert_eq!(value(&name), 17);
    assert_eq!(
        value(&probe),
        0,
        "incomplete native transaction survived crash"
    );
    assert_eq!(
        claim_owner(
            c.begin(&control).unwrap(),
            OwnerToken::new([63; 32]).unwrap()
        ),
        Err(OwnerError::Busy)
    );
    assert_eq!(
        with_owner(c.begin(&control).unwrap(), old, |tx| tx
            .execute(&update(&name), vec![99_i64.into()])
            .map(|_| ())),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        release_owner(c.begin(&control).unwrap(), old),
        Err(OwnerError::Stale)
    );
    assert_eq!(value(&name), 17);
    with_owner(c.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), vec![42_i64.into()]).map(|_| ())
    })
    .unwrap();
    release_owner(c.begin(&control).unwrap(), owner).unwrap();
    let next = claim_owner(
        c.begin(&control).unwrap(),
        OwnerToken::new([63; 32]).unwrap(),
    )
    .unwrap();
    assert_eq!(next.generation(), 3);
    drop(c);
    let mut fresh = connect();
    let mut tx = fresh.begin(&control).unwrap();
    assert_eq!(tx.lock_owner().unwrap(), next.state());
    tx.rollback().unwrap();
    assert_eq!(value(&name), 42);
    println!(
        "Native process recovery: incomplete={incomplete}; uptime {}→{}; generation2/data17 and generation3/data42 exact; stale guards refused",
        before.1, after.1
    );
}
