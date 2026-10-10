//! In-memory acknowledged ownership survives controlled same-volume native graceful restart.
use crate::fixture::{self, Fixture};
use rom_oracle::Value;
use rom_sql_core::{
    OwnerError, OwnerTransaction, claim_owner, release_owner, takeover_owner, with_owner,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    thread,
    time::{Duration, Instant},
};
fn signal(directory: &Path, name: &str, content: &[u8]) {
    let temporary = directory.join(format!("{name}.tmp"));
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .unwrap();
    f.write_all(content).unwrap();
    f.sync_all().unwrap();
    drop(f);
    fs::rename(temporary, directory.join(name)).unwrap();
}
pub(crate) fn run(directory: &Path) {
    assert!(directory.is_dir());
    let f = Fixture::new();
    let mut c = fixture::connection();
    let g1 = claim_owner(c.begin(&f.control).unwrap(), fixture::token(61)).unwrap();
    with_owner(c.begin(&f.control).unwrap(), g1, |tx| {
        tx.execute(&f.update(), &[Value::Integer(Some(17))])
            .map(|_| ())
    })
    .unwrap();
    let g2 = takeover_owner(c.begin(&f.control).unwrap(), g1.state(), fixture::token(62)).unwrap();
    assert_eq!(g2.generation(), 2);
    assert_eq!(f.value(), 17);
    drop(c); // Native clients close; acknowledged guards remain live in this process without release.
    signal(directory, "prepared", b"ready\n");
    let end = Instant::now() + Duration::from_secs(240);
    while !directory.join("restarted").exists() {
        assert!(Instant::now() < end, "bounded owned fixture restart");
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(fs::read(directory.join("restarted")).unwrap(), b"resume\n");
    let mut c = fixture::connection();
    let mut tx = c.begin(&f.control).unwrap();
    assert_eq!(tx.lock_owner().unwrap(), g2.state());
    tx.rollback().unwrap();
    assert_eq!(f.value(), 17);
    assert_eq!(
        claim_owner(c.begin(&f.control).unwrap(), fixture::token(63)),
        Err(OwnerError::Busy)
    );
    assert_eq!(
        with_owner(c.begin(&f.control).unwrap(), g1, |tx| tx
            .execute(&f.update(), &[Value::Integer(Some(99))])
            .map(|_| ())),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        release_owner(c.begin(&f.control).unwrap(), g1),
        Err(OwnerError::Stale)
    );
    assert_eq!(f.value(), 17);
    with_owner(c.begin(&f.control).unwrap(), g2, |tx| {
        tx.execute(&f.update(), &[Value::Integer(Some(42))])
            .map(|_| ())
    })
    .unwrap();
    release_owner(c.begin(&f.control).unwrap(), g2).unwrap();
    let g3 = claim_owner(c.begin(&f.control).unwrap(), fixture::token(63)).unwrap();
    assert_eq!(g3.generation(), 3);
    assert_eq!(f.value(), 42);
    release_owner(c.begin(&f.control).unwrap(), g3).unwrap();
    signal(
        directory,
        "verified",
        b"generation2 retained; generation1 stale; generation3 acknowledged; value42\n",
    );
    println!(
        "Oracle graceful native restart preserved exact generation2/data17; stale1 denied; fresh3/data42 acknowledged"
    );
}
