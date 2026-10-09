//! Keep acknowledged guards in memory; inspect exact native state after a real restart.
use crate::fixture::*;
use rom_mssql::{Connection, Deadlines};
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

pub fn run(directory: &Path) {
    assert!(directory.is_dir());
    let (name, control) = table("restart");
    let mut c = connect();
    let old = claim_owner(
        c.begin(&control).unwrap(),
        OwnerToken::new([61; 32]).unwrap(),
    )
    .unwrap();
    with_owner(c.begin(&control).unwrap(), old, |tx| {
        tx.execute(&update(&name), &[&17_i64])
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
    drop(c); // No socket or active transaction survives the restart; acknowledged guards do.
    let mut ready = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.join("prepared.tmp"))
        .unwrap();
    ready.write_all(b"ready\n").unwrap();
    ready.sync_all().unwrap();
    drop(ready);
    fs::rename(directory.join("prepared.tmp"), directory.join("prepared")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(65);
    while !directory.join("restarted").exists() {
        assert!(Instant::now() < deadline, "owned restart did not finish");
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        fs::read_to_string(directory.join("restarted")).unwrap(),
        "resume\n"
    );
    // Explicit fixture readiness retry only. No owner transition or write is retried.
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut c = loop {
        match Connection::connect(
            config(),
            Deadlines::new(
                Duration::from_secs(1),
                Duration::from_secs(4),
                Duration::from_millis(500),
            )
            .unwrap(),
        ) {
            Ok(c) => break c,
            Err(_) => {
                assert!(Instant::now() < deadline, "SQL Server did not become ready");
                thread::sleep(Duration::from_millis(50));
            }
        }
    };
    assert_eq!(
        scalar("SELECT CAST(delayed_durability AS BIGINT) FROM sys.databases WHERE name=DB_NAME()"),
        0
    );
    let mut inspected = c.begin(&control).unwrap();
    assert_eq!(inspected.lock_owner().unwrap(), owner.state());
    inspected.rollback().unwrap();
    assert_eq!(value(&name), 17);
    assert_eq!(
        claim_owner(
            c.begin(&control).unwrap(),
            OwnerToken::new([63; 32]).unwrap()
        ),
        Err(OwnerError::Busy)
    );
    assert_eq!(
        with_owner(c.begin(&control).unwrap(), old, |tx| tx
            .execute(&update(&name), &[&99_i64])),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        release_owner(c.begin(&control).unwrap(), old),
        Err(OwnerError::Stale)
    );
    assert_eq!(value(&name), 17);
    with_owner(c.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&42_i64])
    })
    .unwrap();
    release_owner(c.begin(&control).unwrap(), owner).unwrap();
    let successor = claim_owner(
        c.begin(&control).unwrap(),
        OwnerToken::new([63; 32]).unwrap(),
    )
    .unwrap();
    assert_eq!(successor.generation(), 3);
    drop(c);
    let mut reopened = connect();
    let mut tx = reopened.begin(&control).unwrap();
    assert_eq!(tx.lock_owner().unwrap(), successor.state());
    tx.rollback().unwrap();
    assert_eq!(value(&name), 42);
    println!(
        "Same-volume SQL Server restart: exact generation2 and data17 retained; stale guard denied; explicit generation3 and data42 retained"
    );
}
