use crate::fixture::*;
use rom_cockroach::{Connection, ControlTable, Deadlines};
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
    let version: String = raw().query_one("SELECT version()", &[]).unwrap().get(0);
    assert!(version.starts_with("CockroachDB CCL v26.3."));
    println!("Native verified CockroachDB26.3 profile");
    transitions();
    rollback();
    malformed();
    race();
    ordered_takeover();
    retry_rejection();
    ambiguity_rejection();
    malformed_protocol_code();
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
            tx.execute("UPDATE rom_missing_native_qualification SET x=1", &[])
                .map(|_| ())
        }),
        Err(OwnerError::Uninitialized)
    );
    assert_eq!(value(&name), 0);
    let mut tx = c.begin(&control).unwrap();
    tx.lock_owner().unwrap();
    tx.execute(&update(&name), &[&88_i64]).unwrap();
    assert_eq!(
        tx.execute("UPDATE rom_missing_native_qualification SET x=1", &[]),
        Err(OwnerError::Uninitialized)
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
pub fn malformed() {
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
    raw().batch_execute(&format!("CREATE TABLE public.{duplicate} AS SELECT * FROM public.{name}; INSERT INTO public.{duplicate} SELECT * FROM public.{name}")).unwrap();
    let duplicate = ControlTable::new("public", &duplicate, ID).unwrap();
    assert_eq!(
        claim_owner(c.begin(&duplicate).unwrap(), token(5)),
        Err(OwnerError::Invalid)
    );
    let (active_name, _) = table("dupactive");
    let active_duplicate = format!("d_{active_name}");
    raw().batch_execute(&format!("UPDATE public.{active_name} SET generation=1,owner_token=decode(repeat('22',32),'hex'); CREATE TABLE public.{active_duplicate} AS SELECT * FROM public.{active_name}; INSERT INTO public.{active_duplicate} SELECT * FROM public.{active_name}")).unwrap();
    let active_duplicate = ControlTable::new("public", &active_duplicate, ID).unwrap();
    assert_eq!(
        claim_owner(c.begin(&active_duplicate).unwrap(), token(5)),
        Err(OwnerError::Invalid)
    );
    assert!(c.is_available());
    let (empty_name, empty_control) = table("empty");
    raw()
        .batch_execute(&format!("DELETE FROM public.{empty_name}"))
        .unwrap();
    assert_eq!(
        claim_owner(c.begin(&empty_control).unwrap(), token(5)),
        Err(OwnerError::Uninitialized)
    );
    let typed = format!("type_{name}");
    raw().batch_execute(&format!("CREATE TABLE public.{typed}(singleton_key INT4 PRIMARY KEY,format_version STRING NOT NULL,store_identity BYTES NOT NULL,generation INT8 NOT NULL,owner_token BYTES); INSERT INTO public.{typed} VALUES(1,'1',decode('{}','hex'),0,NULL)", "11".repeat(32))).unwrap();
    let typed = ControlTable::new("public", &typed, ID).unwrap();
    assert_eq!(
        claim_owner(c.begin(&typed).unwrap(), token(5)),
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
            let result = claim_owner(c.begin(&ctl).unwrap(), token(b));
            if result == Err(OwnerError::Unavailable) {
                claim_owner(c.begin(&ctl).unwrap(), token(b))
            } else {
                result
            }
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
pub fn ordered_takeover() {
    let (name, control) = table("ordering");
    let mut old = connect();
    let owner = claim_owner(old.begin(&control).unwrap(), token(14)).unwrap();
    let mut held = old.begin(&control).unwrap();
    held.lock_owner().unwrap(); // No write before inspection: qualify SELECT's durable lock itself.
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
        takeover_owner(c.begin(&ctl).unwrap(), owner.state(), token(15))
    });
    let mut monitor = raw();
    monitor
        .batch_execute("SET allow_unsafe_internals=true")
        .unwrap();
    let end = Instant::now() + Duration::from_millis(1500);
    loop {
        let row = monitor.query_one("SELECT count(*) FILTER (WHERE granted AND upper(durability)='REPLICATED' AND upper(lock_strength)='EXCLUSIVE'), count(*) FILTER (WHERE NOT granted) FROM crdb_internal.cluster_locks WHERE database_name='rom_extras' AND schema_name='public' AND table_name=$1 AND contended", &[&name]).unwrap();
        if row.get::<_, i64>(0) == 1 && row.get::<_, i64>(1) == 1 {
            break;
        }
        assert!(
            Instant::now() < end,
            "exact replicated SELECT lock and waiter not observed"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let wrong: i64 = monitor.query_one("SELECT count(*) FROM crdb_internal.cluster_locks WHERE table_name='nonexistent_unique_qualification_table' AND contended", &[]).unwrap().get(0);
    assert_eq!(wrong, 0);
    held.execute(&update(&name), &[&17_i64]).unwrap();
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
    println!(
        "Native replicated SELECT lock, exact waiter, ordered takeover and surviving fence: passed"
    );
}
fn retry_rejection() {
    let (name, control) = table("retry");
    let mut native = raw();
    native
        .batch_execute("SET allow_unsafe_internals=true; BEGIN ISOLATION LEVEL SERIALIZABLE")
        .unwrap();
    let error = native
        .query("SELECT crdb_internal.force_retry('1s'::INTERVAL)", &[])
        .unwrap_err();
    assert_eq!(error.code().map(|code| code.code()), Some("40001"));
    native.batch_execute("ROLLBACK").unwrap();
    let mut cfg = config();
    cfg.options("-c allow_unsafe_internals=true");
    let mut c = Connection::connect_loopback(cfg, limits()).unwrap();
    let owner = claim_owner(c.begin(&control).unwrap(), token(13)).unwrap();
    assert_eq!(
        with_owner(c.begin(&control).unwrap(), owner, |tx| {
            tx.execute(&update(&name), &[&99_i64])?;
            tx.execute("SELECT crdb_internal.force_retry('1s'::INTERVAL)", &[])
                .map(|_| ())
        }),
        Err(OwnerError::Unavailable)
    );
    assert!(c.is_available());
    assert_eq!(value(&name), 0);
    with_owner(c.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&42_i64]).map(|_| ())
    })
    .unwrap();
    assert_eq!(value(&name), 42);
    println!(
        "Native40001 confirmed full rollback, fresh ownership reread and explicit host retry: passed"
    );
}
pub fn ambiguity_rejection() {
    let (name, _) = table("ambiguity");
    let view = format!("v_{name}");
    let mut observer = raw();
    observer
        .batch_execute("SET allow_unsafe_internals=true")
        .unwrap();
    observer.batch_execute(&format!("CREATE VIEW public.{view} AS SELECT singleton_key,crdb_internal.force_error('40003','owned synthetic ambiguity')::INT4 AS format_version,store_identity,generation,owner_token FROM public.{name}")).unwrap();
    observer
        .batch_execute("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .unwrap();
    let error = observer
        .query(
            &format!("SELECT format_version FROM public.{view} WHERE singleton_key=1 FOR UPDATE"),
            &[],
        )
        .unwrap_err();
    assert_eq!(error.code().map(|code| code.code()), Some("40003"));
    observer.batch_execute("ROLLBACK").unwrap();
    let mut cfg = config();
    cfg.options("-c allow_unsafe_internals=true");
    let mut c = Connection::connect_loopback(cfg, limits()).unwrap();
    let control = ControlTable::new("public", &view, ID).unwrap();
    let mut tx = c.begin(&control).unwrap();
    assert_eq!(tx.lock_owner(), Err(OwnerError::Unknown));
    drop(tx);
    assert!(!c.is_available());
    assert_eq!(c.begin(&control).unwrap_err(), OwnerError::Unavailable);
    assert_eq!(value(&name), 0);
    println!(
        "Native developer-injected40003 during ownership read: Unknown and retired; no distributed ambiguity claim"
    );
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
    for dribble in [false, true] {
        transport_delivery(dribble);
    }
}
fn transport_delivery(dribble: bool) {
    let (name, control) = table("wire");
    let mut direct = connect();
    let owner = claim_owner(direct.begin(&control).unwrap(), token(9)).unwrap();
    drop(direct);
    let relay = crate::wire_proxy::Relay::new(55457);
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
    let started = Instant::now();
    let result = with_owner(c.begin(&control).unwrap(), owner, |tx| {
        tx.execute(&update(&name), &[&17_i64])?;
        relay.arm_delivery(!dribble, dribble);
        start.send(()).unwrap();
        Ok(())
    });
    let returned = Instant::now();
    assert!(
        started.elapsed() >= Duration::from_millis(1800)
            && started.elapsed() < Duration::from_secs(3)
    );
    assert_eq!(result, Err(OwnerError::Unknown));
    assert!(!c.is_available());
    assert_eq!(c.begin(&control).unwrap_err(), OwnerError::Unavailable);
    let (v, observed) = observer.join().unwrap();
    assert_eq!(v, 17);
    assert!(observed < returned);
    let counts = relay.finish_after_client_close();
    assert!(counts.requests_after_arm > 0 && counts.responses_after_arm > 0);
    if dribble {
        assert!(
            counts.delivered_after_arm > 0
                && counts.delivered_after_arm < counts.responses_after_arm
        );
    } else {
        assert!(counts.suppressed > 0);
        assert_eq!(counts.delivered_after_arm, 0);
        assert_eq!(counts.suppressed, counts.responses_after_arm);
    }
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
    let relay = crate::wire_proxy::Relay::new(55457);
    let c = Connection::connect_loopback(proxy_config(relay.port), limits()).unwrap();
    rt.block_on(async { drop(c) });
    let counts = relay.finish_after_client_close();
    assert!(counts.client_closed);
    println!("Active Tokio refusal and async handoff Drop: passed");
}

pub fn malformed_protocol_code() {
    let mut cfg = config();
    cfg.options("-c allow_unsafe_internals=true");
    let mut wire = rom_pgwire::Connection::connect_loopback(
        cfg,
        rom_sql_core::OperationDeadlines::new(Duration::from_secs(3), Duration::from_secs(4))
            .unwrap(),
    )
    .unwrap();
    let error = wire
        .execute(
            "SELECT crdb_internal.force_error($1::STRING,$2::STRING)",
            &[
                &"synthetic_private_code_canary",
                &"synthetic_private_message_canary",
            ],
        )
        .unwrap_err();
    assert_eq!(error, rom_pgwire::Error::Unknown);
    assert!(!wire.is_available());
    assert!(!format!("{error:?} {error}").contains("canary"));
    println!(
        "Native developer-injected malformed protocol code discarded without message retention: passed"
    );
}
