//! Native race and recovery assertions; SDK acknowledgement suppression is explicitly simulated.
use crate::driver::*;
use rom_sql_core::{
    Executor, ExecutorError, OwnerError, OwnerTransaction, claim_owner, release_owner,
    takeover_owner, with_owner,
};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
const WAIT: Duration = Duration::from_secs(5);
pub fn run() {
    let mut client = connect();
    let row = client
        .query_one(
            "SELECT current_setting('fsync'),current_setting('synchronous_commit'),version()",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "on");
    assert_eq!(row.get::<_, String>(1), "on");
    println!("Backend: {}", row.get::<_, String>(2));
    stale();
    assert!(competing_claims(false));
    ordered_takeover();
    rollback();
    unknown();
    deadline();
    invalid();
    lock_timeout();
}
fn lock_timeout() {
    let mut client = connect();
    let table = provision(&mut client, "lock_timeout");
    let owner = claim_owner(Tx::begin(&mut client, &table), token(1)).unwrap();
    let mut held = Tx::begin(&mut client, &table);
    assert_eq!(held.lock_owner().unwrap(), owner.state());
    let mut other = connect();
    let start = Instant::now();
    assert_eq!(
        with_owner(Tx::begin(&mut other, &table), owner, |tx| tx
            .write_value(99)),
        Err(OwnerError::Unavailable)
    );
    assert!(start.elapsed() >= Duration::from_secs(1) && start.elapsed() < WAIT);
    held.rollback().unwrap();
    assert_eq!(value(&mut other, &table), 0);
    with_owner(Tx::begin(&mut other, &table), owner, |tx| {
        tx.write_value(42)
    })
    .unwrap();
    println!("native lock timeout rolls back and permits explicit subsequent use: passed");
}
fn stale() {
    let mut owner_connection = connect();
    let table = provision(&mut owner_connection, "stale");
    let first = claim_owner(Tx::begin(&mut owner_connection, &table), token(1)).unwrap();
    let mut pooled = connect();
    with_owner(Tx::begin(&mut pooled, &table), first, |tx| {
        tx.write_value(17)
    })
    .unwrap();
    drop(owner_connection);
    let mut new_owner = connect();
    let observed = inspect(&mut new_owner, &table);
    let second = takeover_owner(Tx::begin(&mut new_owner, &table), observed, token(2)).unwrap();
    assert_eq!(second.generation(), 2);
    assert_eq!(
        with_owner(Tx::begin(&mut pooled, &table), first, |tx| tx
            .write_value(99)),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        release_owner(Tx::begin(&mut pooled, &table), first),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        takeover_owner(Tx::begin(&mut pooled, &table), observed, token(3)),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        claim_owner(Tx::begin(&mut pooled, &table), token(3)),
        Err(OwnerError::Busy)
    );
    assert_eq!(value(&mut pooled, &table), 17);
    release_owner(Tx::begin(&mut new_owner, &table), second).unwrap();
    assert_eq!(
        claim_owner(Tx::begin(&mut pooled, &table), token(1))
            .unwrap()
            .generation(),
        3
    );
    println!("stale surviving connection, exact release and monotonic reclaim: passed");
}
pub fn setup_failure_probe() {
    assert!(!competing_claims(true));
    println!("bounded second connection setup failure: passed");
}
fn competing_claims(inject_failure: bool) -> bool {
    let mut client = connect();
    let table = provision(&mut client, "race");
    let (ready_tx, ready_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let mut releases = Vec::new();
    let mut jobs = Vec::new();
    for byte in 1..=2 {
        let table = table.clone();
        let ready = ready_tx.clone();
        let result = result_tx.clone();
        let (release, admitted) = mpsc::channel();
        releases.push(release);
        jobs.push(thread::spawn(move || {
            let prepared = std::panic::catch_unwind(|| {
                if inject_failure && byte == 2 {
                    panic!("synthetic second connection setup failure");
                }
                connect()
            });
            if ready.send(prepared.is_ok()).is_err() {
                return;
            }
            let outcome = match prepared {
                Ok(mut connection) if admitted.recv_timeout(WAIT).unwrap_or(false) => {
                    Some(claim_owner(Tx::begin(&mut connection, &table), token(byte)))
                }
                _ => None,
            };
            let _ = result.send(outcome);
        }));
    }
    drop(ready_tx);
    drop(result_tx);
    let deadline = Instant::now() + WAIT;
    let ready = (0..2).all(|_| {
        ready_rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) == Ok(true)
    });
    for release in releases {
        let _ = release.send(ready);
    }
    let mut results = Vec::new();
    for _ in 0..2 {
        let Ok(result) = result_rx.recv_timeout(WAIT) else {
            return false;
        };
        results.push(result);
    }
    for job in jobs {
        if job.join().is_err() {
            return false;
        }
    }
    if !ready {
        return false;
    }
    let results: Vec<_> = results.into_iter().map(Option::unwrap).collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == Err(OwnerError::Busy))
            .count(),
        1
    );
    assert_eq!(inspect(&mut client, &table).generation(), 1);
    println!("two independent native claims: one owner, one Busy");
    true
}
fn ordered_takeover() {
    let mut client = connect();
    let table = provision(&mut client, "ordering");
    let first = claim_owner(Tx::begin(&mut client, &table), token(1)).unwrap();
    let mut tx = Tx::begin(&mut client, &table);
    assert_eq!(tx.lock_owner().unwrap(), first.state());
    tx.write_value(17).unwrap();
    let (pid_tx, pid_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let other_table = table.clone();
    let job = thread::spawn(move || {
        let mut client = connect();
        let pid: i32 = client
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        pid_tx.send(pid).unwrap();
        let result = takeover_owner(
            Tx::begin(&mut client, &other_table),
            first.state(),
            token(2),
        );
        result_tx.send(result).unwrap();
    });
    let pid = pid_rx.recv_timeout(WAIT).unwrap();
    let mut observer = connect();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let row = observer
            .query_one(
                "SELECT wait_event_type FROM pg_stat_activity WHERE pid=$1",
                &[&pid],
            )
            .unwrap();
        if row.get::<_, Option<String>>(0).as_deref() == Some("Lock") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "takeover did not reach native row-lock wait"
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(
        result_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    tx.commit().unwrap();
    let second = result_rx.recv_timeout(WAIT).unwrap().unwrap();
    job.join().unwrap();
    assert_eq!(value(&mut observer, &table), 17);
    assert_eq!(
        with_owner(Tx::begin(&mut observer, &table), first, |tx| tx
            .write_value(99)),
        Err(OwnerError::Stale)
    );
    with_owner(Tx::begin(&mut observer, &table), second, |tx| {
        tx.write_value(42)
    })
    .unwrap();
    assert_eq!(value(&mut observer, &table), 42);
    println!("native wait observed; old commit precedes acknowledged takeover: passed");
}
fn rollback() {
    let mut client = connect();
    let table = provision(&mut client, "rollback");
    let owner = claim_owner(Tx::begin(&mut client, &table), token(1)).unwrap();
    let result = with_owner(Tx::begin(&mut client, &table), owner, |tx| {
        tx.write_value(99)?;
        tx.inner
            .query_one("SELECT 1/0", &[])
            .map_err(|_| OwnerError::Unavailable)?;
        Ok(())
    });
    assert_eq!(result, Err(OwnerError::Unavailable));
    assert_eq!(value(&mut client, &table), 0);
    println!("statement failure rolls back whole protected transaction: passed");
}
fn unknown() {
    let mut client = connect();
    let table = provision(&mut client, "unknown");
    let mut tx = Tx::begin(&mut client, &table);
    tx.lose_ack = true;
    assert_eq!(claim_owner(tx, token(1)), Err(OwnerError::Unknown));
    let state = inspect(&mut client, &table);
    assert_eq!(state.generation(), 1);
    assert_eq!(
        claim_owner(Tx::begin(&mut client, &table), token(2)),
        Err(OwnerError::Busy)
    );
    let owner = takeover_owner(Tx::begin(&mut client, &table), state, token(2)).unwrap();
    let mut tx = Tx::begin(&mut client, &table);
    tx.lose_ack = true;
    assert_eq!(
        with_owner(tx, owner, |tx| tx.write_value(42)),
        Err(OwnerError::Unknown)
    );
    let mut reopened = connect();
    assert_eq!(value(&mut reopened, &table), 42);
    assert_eq!(inspect(&mut reopened, &table), owner.state());
    println!(
        "actual commit with simulated SDK acknowledgement suppression: passed; not a wire fault"
    );
}
fn deadline() {
    let mut client = connect();
    let table = provision(&mut client, "deadline");
    let observed_table = table.clone();
    let executor = Executor::spawn(2, || Ok(connect())).unwrap();
    let (start_tx, start_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let ticket = executor
        .submit(move |client| {
            let owner = claim_owner(Tx::begin(client, &table), token(1))?;
            with_owner(Tx::begin(client, &table), owner, |tx| {
                tx.write_value(42)?;
                start_tx.send(()).unwrap();
                release_rx.recv_timeout(WAIT).unwrap();
                Ok(owner)
            })
        })
        .unwrap();
    start_rx.recv_timeout(WAIT).unwrap();
    assert_eq!(
        ticket.wait_timeout(Duration::ZERO),
        Err(ExecutorError::Unknown)
    );
    release_tx.send(()).unwrap();
    let owner = ticket.wait_timeout(WAIT).unwrap().unwrap();
    executor.shutdown().unwrap();
    assert_eq!(value(&mut client, &observed_table), 42);
    assert_eq!(inspect(&mut client, &observed_table), owner.state());
    println!("executor waiting timeout preserves later native protected commit: passed");
}
fn invalid() {
    let mut client = connect();
    let table = provision(&mut client, "invalid");
    assert_eq!(
        claim_owner(Tx::begin(&mut client, &table).wrong_identity(), token(1)),
        Err(OwnerError::Invalid)
    );
    assert_eq!(inspect(&mut client, &table).generation(), 0);
    client
        .execute(&format!("UPDATE {table} SET format=2"), &[])
        .unwrap();
    assert_eq!(
        claim_owner(Tx::begin(&mut client, &table), token(1)),
        Err(OwnerError::UnsupportedFormat)
    );
    let generation: i64 = client
        .query_one(&format!("SELECT generation FROM {table}"), &[])
        .unwrap()
        .get(0);
    assert_eq!(generation, 0);
    let exhausted = provision(&mut client, "exhausted");
    client
        .execute(
            &format!("UPDATE {exhausted} SET generation=$1"),
            &[&i64::MAX],
        )
        .unwrap();
    assert_eq!(
        claim_owner(Tx::begin(&mut client, &exhausted), token(1)),
        Err(OwnerError::Exhausted)
    );
    assert_eq!(
        inspect(&mut client, &exhausted).generation(),
        i64::MAX as u64
    );
    assert_eq!(
        claim_owner(
            Tx::begin(&mut client, "extras_owner_absent_table"),
            token(1)
        ),
        Err(OwnerError::Uninitialized)
    );
    println!("format, identity, absent table and generation bounds refuse writes: passed");
}
