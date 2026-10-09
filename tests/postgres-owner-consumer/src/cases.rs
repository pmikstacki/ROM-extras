use crate::fixture::*;
use rom_postgres::{Connection, ControlTable, Deadlines};
use rom_sql_core::{
    OwnerError, OwnerToken, OwnerTransaction, claim_owner, release_owner, takeover_owner,
    with_owner,
};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
fn token(b: u8) -> OwnerToken {
    OwnerToken::new([b; 32]).unwrap()
}
pub fn run() {
    let row = raw()
        .query_one(
            "SELECT current_setting('fsync'),current_setting('synchronous_commit'),version()",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "on");
    assert_eq!(row.get::<_, String>(1), "on");
    println!("Native version: {}", row.get::<_, String>(2));
    transitions();
    rollback();
    malformed();
    race();
    ordered_takeover();
    lock_timeout();
    transport();
    runtime_boundary();
}
fn transitions() {
    let (name, control) = table("fence");
    let mut first = connect();
    let owner = claim_owner(first.begin(&control).unwrap(), token(1)).unwrap();
    let mut pooled = connect();
    with_owner(pooled.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&17_i64]).map(|_| ())
    })
    .unwrap();
    drop(first);
    let mut other = connect();
    let successor =
        takeover_owner(other.begin(&control).unwrap(), owner.state(), token(2)).unwrap();
    assert_eq!(successor.generation(), 2);
    assert_eq!(
        with_owner(pooled.begin(&control).unwrap(), owner, |tx| tx
            .execute(&update(&name), &[&99_i64])
            .map(|_| ())),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        release_owner(pooled.begin(&control).unwrap(), owner),
        Err(OwnerError::Stale)
    );
    assert_eq!(value(&name), 17);
    with_owner(pooled.begin(&control).unwrap(), successor, |tx| {
        tx.execute(&update(&name), &[&42_i64]).map(|_| ())
    })
    .unwrap();
    release_owner(other.begin(&control).unwrap(), successor).unwrap();
    let reclaimed = claim_owner(other.begin(&control).unwrap(), token(3)).unwrap();
    assert_eq!(reclaimed.generation(), 3);
    assert_eq!(value(&name), 42);
    assert!(!format!("{pooled:?} {control:?}").contains(&name));
    println!("Native exact transitions and surviving stale connection: passed");
}
fn rollback() {
    let (name, control) = table("rollback");
    let mut c = connect();
    let owner = claim_owner(c.begin(&control).unwrap(), token(4)).unwrap();
    assert_eq!(
        with_owner(c.begin(&control).unwrap(), owner, |tx| {
            tx.execute(&update(&name), &[&99_i64])?;
            tx.execute(
                "DO $$ BEGIN RAISE EXCEPTION 'native rejection'; END $$",
                &[],
            )
            .map(|_| ())
        }),
        Err(OwnerError::Unavailable)
    );
    assert_eq!(value(&name), 0);
    let mut tx = c.begin(&control).unwrap();
    tx.lock_owner().unwrap();
    tx.execute(&update(&name), &[&88_i64]).unwrap();
    assert_eq!(
        tx.execute(
            "DO $$ BEGIN RAISE EXCEPTION 'caught native rejection'; END $$",
            &[]
        ),
        Err(OwnerError::Unavailable)
    );
    assert_eq!(tx.lock_owner(), Err(OwnerError::Invalid));
    assert_eq!(tx.commit(), Err(OwnerError::Invalid));
    assert_eq!(value(&name), 0);
    {
        let mut tx = c.begin(&control).unwrap();
        tx.lock_owner().unwrap();
        tx.execute(&update(&name), &[&77_i64]).unwrap();
    }
    assert_eq!(value(&name), 0);
    with_owner(c.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&42_i64]).map(|_| ())
    })
    .unwrap();
    assert_eq!(value(&name), 42);
    println!("Native prefix rollback, caught-error poison and Drop: passed");
}
fn malformed() {
    let (name, control) = table("invalid");
    let mut c = connect();
    let wrong = ControlTable::new("public", &name, [19; 32]).unwrap();
    assert_eq!(
        claim_owner(c.begin(&wrong).unwrap(), token(5)),
        Err(OwnerError::Invalid)
    );
    for (sql, error) in [
        ("format_version=2", OwnerError::UnsupportedFormat),
        ("generation=-1", OwnerError::Invalid),
        (
            "generation=0,owner_token=decode(repeat('11',32),'hex')",
            OwnerError::Invalid,
        ),
        ("store_identity=decode('11','hex')", OwnerError::Invalid),
        (
            "format_version=1,owner_token=decode('00','hex'),generation=1",
            OwnerError::Invalid,
        ),
        (
            "owner_token=decode(repeat('00',32),'hex')",
            OwnerError::Invalid,
        ),
        (
            "owner_token=decode(repeat('11',1048576),'hex')",
            OwnerError::Invalid,
        ),
        (
            "owner_token=NULL,store_identity=decode(repeat('11',1048576),'hex')",
            OwnerError::Invalid,
        ),
    ] {
        // Each malformed field starts from known valid idle state, avoiding masked checks.
        raw().batch_execute(&format!("UPDATE public.{name} SET format_version=1,store_identity=decode('{}','hex'),generation=0,owner_token=NULL", "11".repeat(32))).unwrap();
        raw()
            .batch_execute(&format!("UPDATE public.{name} SET {sql}"))
            .unwrap();
        assert_eq!(
            claim_owner(c.begin(&control).unwrap(), token(5)),
            Err(error)
        );
    }
    raw().batch_execute(&format!("UPDATE public.{name} SET store_identity=decode('{}','hex'),owner_token=NULL,generation=9223372036854775807","11".repeat(32))).unwrap();
    assert_eq!(
        claim_owner(c.begin(&control).unwrap(), token(5)),
        Err(OwnerError::Exhausted)
    );
    let missing = ControlTable::new("public", &format!("missing_{name}"), ID).unwrap();
    assert_eq!(
        claim_owner(c.begin(&missing).unwrap(), token(5)),
        Err(OwnerError::Uninitialized)
    );
    let duplicate = format!("dup_{name}");
    raw().batch_execute(&format!("CREATE TABLE public.{duplicate} AS TABLE public.{name}; INSERT INTO public.{duplicate} SELECT * FROM public.{name}")).unwrap();
    let duplicate = ControlTable::new("public", &duplicate, ID).unwrap();
    assert_eq!(
        claim_owner(c.begin(&duplicate).unwrap(), token(5)),
        Err(OwnerError::Invalid)
    );
    println!(
        "Native independently malformed metadata, missing/duplicate rows and exhausted generation: passed"
    );
}
fn race() {
    let (name, _) = table("race");
    let mut joins = Vec::new();
    let mut releases = Vec::new();
    let (ready, prepared) = mpsc::channel();
    for b in [6, 7] {
        let name = name.clone();
        let ready = ready.clone();
        let (go, released) = mpsc::sync_channel(1);
        releases.push(go);
        joins.push(thread::spawn(move || {
            let mut c = connect();
            let ctl = ControlTable::new("public", &name, ID).unwrap();
            ready.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(3)).unwrap();
            claim_owner(c.begin(&ctl).unwrap(), token(b))
        }));
    }
    for _ in 0..2 {
        prepared.recv_timeout(Duration::from_secs(3)).unwrap();
    }
    for go in releases {
        go.send(()).unwrap();
    }
    let outcomes = joins
        .into_iter()
        .map(|j| j.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| matches!(r, Err(OwnerError::Busy)))
            .count(),
        1
    );
    println!("Native competing claims: passed");
}
fn ordered_takeover() {
    let (name, control) = table("ordering");
    let mut old = connect();
    let owner = claim_owner(old.begin(&control).unwrap(), token(13)).unwrap();
    let mut held = old.begin(&control).unwrap();
    held.lock_owner().unwrap();
    held.execute(&update(&name), &[&17_i64]).unwrap();
    let other_name = name.clone();
    let waiter = thread::spawn(move || {
        let ctl = ControlTable::new("public", &other_name, ID).unwrap();
        let mut c = Connection::connect_loopback(
            config(),
            Deadlines::new(
                Duration::from_secs(3),
                Duration::from_secs(4),
                Duration::from_secs(3),
                Duration::from_secs(2),
            )
            .unwrap(),
        )
        .unwrap();
        takeover_owner(c.begin(&ctl).unwrap(), owner.state(), token(14))
    });
    let pattern = format!("%FROM \"public\".\"{name}\"%");
    let end = Instant::now() + Duration::from_secs(2);
    let mut monitor = raw();
    loop {
        let count:i64=monitor.query_one("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE $1 AND pid<>pg_backend_pid()",&[&pattern]).unwrap().get(0);
        if count == 1 {
            break;
        }
        assert!(
            Instant::now() < end,
            "native unique control lock waiter not observed"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let wrong:i64=monitor.query_one("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE '%nonexistent_unique_qualification_table%'",&[]).unwrap().get(0);
    assert_eq!(wrong, 0);
    held.commit().unwrap();
    let successor = waiter.join().unwrap().unwrap();
    assert_eq!(successor.generation(), 2);
    assert_eq!(value(&name), 17);
    assert_eq!(
        with_owner(old.begin(&control).unwrap(), owner, |tx| tx
            .execute(&update(&name), &[&99_i64])
            .map(|_| ())),
        Err(OwnerError::Stale)
    );
    with_owner(old.begin(&control).unwrap(), successor, |tx| {
        tx.execute(&update(&name), &[&42_i64]).map(|_| ())
    })
    .unwrap();
    assert_eq!(value(&name), 42);
    println!("Native unique control lock wait, ordered takeover and surviving fence: passed");
}
fn lock_timeout() {
    let (name, control) = table("lock");
    let mut c = connect();
    let owner = claim_owner(c.begin(&control).unwrap(), token(8)).unwrap();
    let mut held = c.begin(&control).unwrap();
    held.lock_owner().unwrap();
    let mut other = connect();
    let begin = Instant::now();
    assert_eq!(
        with_owner(other.begin(&control).unwrap(), owner, |tx| tx
            .execute(&update(&name), &[&99_i64])
            .map(|_| ())),
        Err(OwnerError::Unavailable)
    );
    assert!(begin.elapsed() >= Duration::from_millis(450));
    held.rollback().unwrap();
    assert_eq!(value(&name), 0);
    with_owner(other.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&42_i64]).map(|_| ())
    })
    .unwrap();
    assert_eq!(value(&name), 42);
    println!("Native lock timeout and explicit reuse after confirmed rollback: passed");
}
fn transport() {
    let (name, control) = table("wire");
    let mut direct = connect();
    let owner = claim_owner(direct.begin(&control).unwrap(), token(9)).unwrap();
    drop(direct);
    let relay = crate::wire_proxy::Relay::new(55439);
    let cfg = proxy_config(relay.port);
    let mut c = Connection::connect_loopback(
        cfg,
        Deadlines::new(
            Duration::from_secs(3),
            Duration::from_secs(2),
            Duration::from_secs(1),
            Duration::from_millis(500),
        )
        .unwrap(),
    )
    .unwrap();
    let (start, ready) = mpsc::sync_channel(1);
    let observed_name = name.clone();
    let observer = thread::spawn(move || {
        ready.recv_timeout(Duration::from_secs(3)).unwrap();
        let v = value(&observed_name);
        (v, Instant::now())
    });
    let result = with_owner(c.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&17_i64])?;
        relay.arm(true);
        start.send(()).unwrap();
        Ok(())
    });
    let returned = Instant::now();
    assert_eq!(result, Err(OwnerError::Unknown));
    assert!(!c.is_available());
    assert_eq!(c.begin(&control).unwrap_err(), OwnerError::Unavailable);
    let (v, observed) = observer.join().unwrap();
    assert_eq!(v, 17);
    assert!(observed < returned);
    let counts = relay.finish_after_client_close();
    assert!(counts.requests_after_arm > 0 && counts.suppressed > 0);
    assert_eq!(counts.delivered_after_arm, 0);
    assert_eq!(counts.suppressed, counts.responses_after_arm);
    let mut fresh = connect();
    let mut tx = fresh.begin(&control).unwrap();
    assert_eq!(tx.lock_owner().unwrap(), owner.state());
    tx.rollback().unwrap();
    assert_eq!(value(&name), 17);
    println!(
        "Native committed response loss, whole-I/O timeout and transport retirement: {counts:?}"
    );
}
fn runtime_boundary() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert_eq!(
        rt.block_on(async { Connection::connect_loopback(config(), limits()).unwrap_err() }),
        OwnerError::Invalid
    );
    let c = connect();
    rt.block_on(async { drop(c) });
    println!("Active Tokio refusal and async handoff Drop: passed");
}
