use crate::fixture::*;
use rom_mssql::{Connection, Deadlines};
use rom_sql_core::{
    Executor, ExecutorError, OwnerError, OwnerToken, OwnerTransaction, claim_owner, release_owner,
    takeover_owner, with_owner,
};
use std::{sync::mpsc, thread, time::Duration};
fn token(n: u8) -> OwnerToken {
    OwnerToken::new([n; 32]).unwrap()
}
pub fn run() {
    println!(
        "SQL Server version: {}",
        sql("SELECT CAST(SERVERPROPERTY('ProductVersion') AS VARCHAR(32))")[0][0]
            .get::<&str, _>(0)
            .unwrap()
    );
    assert_eq!(
        scalar("SELECT CAST(delayed_durability AS BIGINT) FROM sys.databases WHERE name=DB_NAME()"),
        0
    );
    stale();
    race();
    ordered();
    errors();
    metadata();
    cleanup();
    worker();
}
fn stale() {
    let (name, table) = table("stale");
    let mut first = connect();
    let owner = claim_owner(first.begin(&table).unwrap(), token(1)).unwrap();
    let mut survivor = connect();
    with_owner(survivor.begin(&table).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&17_i64]).map(|_| ())
    })
    .unwrap();
    drop(first);
    let mut successor = connect();
    let next = takeover_owner(successor.begin(&table).unwrap(), owner.state(), token(2)).unwrap();
    assert_eq!(
        with_owner(survivor.begin(&table).unwrap(), owner, |tx| tx
            .execute(&update(&name), &[&99_i64])),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        release_owner(survivor.begin(&table).unwrap(), owner),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        claim_owner(survivor.begin(&table).unwrap(), token(3)),
        Err(OwnerError::Busy)
    );
    assert_eq!(value(&name), 17);
    release_owner(successor.begin(&table).unwrap(), next).unwrap();
    assert_eq!(
        claim_owner(successor.begin(&table).unwrap(), token(3))
            .unwrap()
            .generation(),
        3
    );
}
fn race() {
    let (_, table) = table("race");
    let mut jobs = Vec::new();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let mut starters = Vec::new();
    for n in 1..=2 {
        let (start_tx, start_rx) = mpsc::channel();
        starters.push(start_tx);
        let table = table.clone();
        let ready = ready_tx.clone();
        let result = result_tx.clone();
        jobs.push(thread::spawn(move || {
            let mut c = connect();
            ready.send(()).unwrap();
            start_rx.recv_timeout(WAIT).unwrap();
            result
                .send(claim_owner(c.begin(&table).unwrap(), token(n)))
                .unwrap();
        }));
    }
    for _ in 0..2 {
        ready_rx.recv_timeout(WAIT).unwrap();
    }
    for start in starters {
        start.send(()).unwrap();
    }
    let results = [
        result_rx.recv_timeout(WAIT).unwrap(),
        result_rx.recv_timeout(WAIT).unwrap(),
    ];
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(OwnerError::Busy))
            .count(),
        1
    );
    for job in jobs {
        job.join().unwrap();
    }
}
fn waiters(name: &str) -> i64 {
    // The identifier came from fixture::table's validated ControlTable constructor.
    scalar(&format!(
        "SELECT CAST(COUNT(*) AS BIGINT) FROM sys.dm_exec_requests r CROSS APPLY sys.dm_exec_sql_text(r.sql_handle) t WHERE r.database_id=DB_ID() AND r.blocking_session_id>0 AND r.wait_type LIKE 'LCK_M_%' AND CHARINDEX('FROM [dbo].[{name}] WITH (UPDLOCK,HOLDLOCK)',t.text)>0"
    ))
}
fn observe_wait(name: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if waiters(name) > 0 {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "native lock wait not observed"
        );
        thread::sleep(Duration::from_millis(10));
    }
}
fn ordered() {
    let (name, table) = table("order");
    let mut old = connect();
    let owner = claim_owner(old.begin(&table).unwrap(), token(1)).unwrap();
    let mut tx = old.begin(&table).unwrap();
    assert_eq!(tx.lock_owner().unwrap(), owner.state());
    tx.execute(&update(&name), &[&17_i64]).unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let control = table.clone();
    let join = thread::spawn(move || {
        let mut c = Connection::connect(
            config(),
            Deadlines::new(WAIT, WAIT, Duration::from_secs(6)).unwrap(),
        )
        .unwrap();
        ready_tx.send(()).unwrap();
        go_rx.recv_timeout(WAIT).unwrap();
        result_tx
            .send(takeover_owner(
                c.begin(&control).unwrap(),
                owner.state(),
                token(2),
            ))
            .unwrap();
    });
    ready_rx.recv_timeout(WAIT).unwrap();
    // An unrelated actual native waiter cannot satisfy this table's barrier.
    let (other_name, other_table) = crate::fixture::table("otherwait");
    let mut other_holder = connect();
    let other_owner = claim_owner(other_holder.begin(&other_table).unwrap(), token(5)).unwrap();
    let mut other_tx = other_holder.begin(&other_table).unwrap();
    assert_eq!(other_tx.lock_owner().unwrap(), other_owner.state());
    let other_control = other_table.clone();
    let distractor = thread::spawn(move || {
        let mut c = Connection::connect(
            config(),
            Deadlines::new(WAIT, WAIT, Duration::from_secs(6)).unwrap(),
        )
        .unwrap();
        claim_owner(c.begin(&other_control).unwrap(), token(6))
    });
    observe_wait(&other_name);
    assert_eq!(
        waiters(&name),
        0,
        "unrelated waiter must not satisfy target barrier"
    );
    other_tx.rollback().unwrap();
    assert_eq!(distractor.join().unwrap(), Err(OwnerError::Busy));
    go_tx.send(()).unwrap();
    observe_wait(&name);
    assert!(result_rx.try_recv().is_err());
    tx.commit().unwrap();
    let next = result_rx.recv_timeout(WAIT).unwrap().unwrap();
    join.join().unwrap();
    assert_eq!(value(&name), 17);
    assert_eq!(next.generation(), 2);
    assert_eq!(
        with_owner(old.begin(&table).unwrap(), owner, |_| Ok(())),
        Err(OwnerError::Stale)
    );
}
fn errors() {
    let (name, table) = table("errors");
    let mut c = connect();
    let owner = claim_owner(c.begin(&table).unwrap(), token(1)).unwrap();
    assert_eq!(
        with_owner(c.begin(&table).unwrap(), owner, |tx| {
            tx.execute(&update(&name), &[&99_i64])?;
            tx.execute("THROW 51000, 'sensitive native server detail', 1", &[])
                .map(|_| ())
        }),
        Err(OwnerError::Unavailable)
    );
    assert_eq!(value(&name), 0);
    with_owner(c.begin(&table).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&42_i64])
    })
    .unwrap();
    let mut held = c.begin(&table).unwrap();
    held.lock_owner().unwrap();
    let mut blocked = connect();
    assert_eq!(
        claim_owner(blocked.begin(&table).unwrap(), token(2)),
        Err(OwnerError::Unavailable)
    );
    held.rollback().unwrap();
    with_owner(blocked.begin(&table).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&43_i64])
    })
    .unwrap();
    assert_eq!(value(&name), 43);
    let mut poisoned = c.begin(&table).unwrap();
    poisoned.lock_owner().unwrap();
    poisoned.execute(&update(&name), &[&99_i64]).unwrap();
    assert_eq!(
        poisoned.execute(
            "UPDATE dbo.rom_extras_nonexistent_owner_fixture SET missing=1",
            &[]
        ),
        Err(OwnerError::Unavailable)
    );
    assert_eq!(poisoned.lock_owner(), Err(OwnerError::Invalid));
    assert_eq!(poisoned.commit(), Err(OwnerError::Invalid));
    assert_eq!(value(&name), 43);
}
fn metadata() {
    let (bounded_name, bounded_table) = table("bounded");
    sql(&format!(
        "ALTER TABLE dbo.[{bounded_name}] ALTER COLUMN store_identity VARBINARY(32) NOT NULL; ALTER TABLE dbo.[{bounded_name}] ALTER COLUMN owner_token VARBINARY(32) NULL"
    ));
    let mut bounded = connect();
    let bounded_owner = claim_owner(bounded.begin(&bounded_table).unwrap(), token(1)).unwrap();
    release_owner(bounded.begin(&bounded_table).unwrap(), bounded_owner).unwrap();

    for (label, patch, expected) in [
        ("format", "format_version=2", OwnerError::UnsupportedFormat),
        ("identity", "store_identity=0x11", OwnerError::Invalid),
        (
            "token",
            "generation=1, owner_token=0x01",
            OwnerError::Invalid,
        ),
        ("negative", "generation=-1", OwnerError::Invalid),
        (
            "longid",
            "store_identity=CONVERT(VARBINARY(MAX),REPLICATE('x',4000))",
            OwnerError::Invalid,
        ),
        (
            "longtoken",
            "generation=1, owner_token=CONVERT(VARBINARY(MAX),REPLICATE('x',4000))",
            OwnerError::Invalid,
        ),
        (
            "zero",
            "generation=1, owner_token=0x0000000000000000000000000000000000000000000000000000000000000000",
            OwnerError::Invalid,
        ),
        (
            "exhaust",
            "generation=9223372036854775807",
            OwnerError::Exhausted,
        ),
    ] {
        let (name, table) = table(label);
        sql(&format!("UPDATE dbo.[{name}] SET {patch}"));
        let mut c = connect();
        assert_eq!(
            claim_owner(c.begin(&table).unwrap(), token(1)),
            Err(expected)
        );
        assert_eq!(value(&name), 0);
    }
    let (name, table) = table("missing");
    sql(&format!("DELETE FROM dbo.[{name}]"));
    assert_eq!(
        claim_owner(connect().begin(&table).unwrap(), token(1)),
        Err(OwnerError::Uninitialized)
    );
}
fn cleanup() {
    let (name, table) = table("cleanup");
    let mut c = connect();
    let owner = claim_owner(c.begin(&table).unwrap(), token(1)).unwrap();
    {
        let mut tx = c.begin(&table).unwrap();
        assert_eq!(
            tx.execute(&update(&name), &[&99_i64]),
            Err(OwnerError::Invalid)
        );
        assert_eq!(tx.lock_owner(), Err(OwnerError::Invalid));
    }
    {
        let mut tx = c.begin(&table).unwrap();
        tx.lock_owner().unwrap();
        tx.execute(&update(&name), &[&99_i64]).unwrap();
    }
    assert_eq!(value(&name), 0);
    with_owner(c.begin(&table).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&42_i64])
    })
    .unwrap();
    let mut short = Connection::connect(
        config(),
        Deadlines::new(WAIT, Duration::from_millis(150), Duration::from_millis(50)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        with_owner(short.begin(&table).unwrap(), owner, |tx| {
            tx.execute(&update(&name), &[&99_i64])?;
            tx.execute("WAITFOR DELAY '00:00:02'", &[]).map(|_| ())
        }),
        Err(OwnerError::Unknown)
    );
    assert!(!short.is_available());
    assert!(matches!(short.begin(&table), Err(OwnerError::Unavailable)));
    let deadline = std::time::Instant::now() + WAIT;
    loop {
        if value(&name) == 42 {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
    }
}
fn worker() {
    let (name, table) = table("worker");
    let executor = Executor::spawn(2, || {
        Connection::connect(
            config(),
            Deadlines::new(WAIT, WAIT, Duration::from_secs(4)).unwrap(),
        )
        .map_err(|_| ExecutorError::Initialization)
    })
    .unwrap();
    let ctl = table.clone();
    let owner = executor
        .execute(move |c| claim_owner(c.begin(&ctl).unwrap(), token(1)))
        .unwrap()
        .unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();
    let ctl = table.clone();
    let statement = update(&name);
    let ticket = executor
        .submit(move |c| {
            with_owner(c.begin(&ctl).unwrap(), owner, |tx| {
                ready_tx.send(()).unwrap();
                go_rx.recv_timeout(WAIT).unwrap();
                tx.execute(&statement, &[&42_i64])
            })
        })
        .unwrap();
    ready_rx.recv_timeout(WAIT).unwrap();
    assert_eq!(
        ticket.wait_timeout(Duration::ZERO),
        Err(ExecutorError::Unknown)
    );
    go_tx.send(()).unwrap();
    executor.shutdown().unwrap();
    assert_eq!(value(&name), 42);
    let mut outside = connect();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        assert!(matches!(outside.begin(&table), Err(OwnerError::Invalid)));
        drop(outside);
        assert!(matches!(
            Connection::connect(
                config(),
                Deadlines::new(WAIT, WAIT, Duration::from_secs(4)).unwrap()
            ),
            Err(OwnerError::Invalid)
        ));
    });
}
