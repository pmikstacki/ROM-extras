//! Actual acknowledged guards survive same-volume native server process death.
use crate::fixture::*;
use rom_cockroach::Connection;
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
fn native_identity() -> (String, String) {
    let mut observer = raw();
    observer
        .batch_execute("SET allow_unsafe_internals=true")
        .unwrap();
    let cluster: String = observer
        .query_one("SELECT crdb_internal.cluster_id()::STRING", &[])
        .unwrap()
        .get(0);
    let row = observer
        .query_one(
            "SELECT node_id,started_at::STRING FROM crdb_internal.kv_node_status LIMIT 1",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 1);
    (cluster, row.get(1))
}
pub fn run(directory: &Path) {
    assert!(directory.is_dir());
    let before = native_identity();
    let (name, control) = table("restart");
    let mut c = connect();
    let old = claim_owner(
        c.begin(&control).unwrap(),
        OwnerToken::new([61; 32]).unwrap(),
    )
    .unwrap();
    with_owner(c.begin(&control).unwrap(), old, |tx| {
        tx.execute(&update(&name), &[&17_i64]).map(|_| ())
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
    drop(c); // Actual acknowledged guards remain; native clients are closed without release.
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
    // Explicit connect readiness retry only; no owner transition or prepared write is retried.
    let end = Instant::now() + Duration::from_secs(30);
    let mut c = loop {
        match Connection::connect_loopback(config(), limits()) {
            Ok(c) => break c,
            Err(_) => {
                assert!(Instant::now() < end, "CockroachDB readiness deadline");
                thread::sleep(Duration::from_millis(50));
            }
        }
    };
    let after = native_identity();
    assert_eq!(before.0, after.0, "different CockroachDB cluster");
    assert_ne!(before.1, after.1, "native node did not restart");
    let mut tx = c.begin(&control).unwrap();
    assert_eq!(tx.lock_owner().unwrap(), owner.state());
    tx.rollback().unwrap();
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
            .execute(&update(&name), &[&99_i64])
            .map(|_| ())),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        release_owner(c.begin(&control).unwrap(), old),
        Err(OwnerError::Stale)
    );
    assert_eq!(value(&name), 17);
    with_owner(c.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&42_i64]).map(|_| ())
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
    let mut fresh = connect();
    let mut tx = fresh.begin(&control).unwrap();
    assert_eq!(tx.lock_owner().unwrap(), successor.state());
    tx.rollback().unwrap();
    assert_eq!(value(&name), 42);
    println!(
        "Same-cluster CockroachDB process crash: changed node start; retained volume; exact generation2/data17 and generation3/data42; stale guards denied"
    );
}
