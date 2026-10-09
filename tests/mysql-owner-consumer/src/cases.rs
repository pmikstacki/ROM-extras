use crate::fixture::*;
use mysql::prelude::Queryable;
use rom_mysql::{Connection, ControlTable, Deadlines};
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
    let version: String = raw().query_first("SELECT VERSION()").unwrap().unwrap();
    println!("Native version: {version}");
    transitions();
    rollback();
    malformed();
    race();
    ordered();
    lock_timeout();
    transport(false);
    transport(true);
    runtime_boundary();
}
fn transitions() {
    let (name, ctl) = table("fence");
    let mut c = connect();
    let owner = claim_owner(c.begin(&ctl).unwrap(), token(1)).unwrap();
    let mut same = c.begin(&ctl).unwrap();
    let state = same.lock_owner().unwrap();
    same.write_owner(state).unwrap();
    same.commit().unwrap();
    let mut other = connect();
    assert_eq!(
        claim_owner(other.begin(&ctl).unwrap(), token(2)),
        Err(OwnerError::Busy)
    );
    with_owner(c.begin(&ctl).unwrap(), owner, |tx| {
        tx.execute(&update(&name), vec![17_i64.into()]).map(|_| ())
    })
    .unwrap();
    let next = takeover_owner(other.begin(&ctl).unwrap(), owner.state(), token(2)).unwrap();
    assert_eq!(next.generation(), 2);
    assert_eq!(
        with_owner(c.begin(&ctl).unwrap(), owner, |tx| tx
            .execute(&update(&name), vec![99_i64.into()])
            .map(|_| ())),
        Err(OwnerError::Stale)
    );
    assert_eq!(
        release_owner(c.begin(&ctl).unwrap(), owner),
        Err(OwnerError::Stale)
    );
    assert_eq!(value(&name), 17);
    with_owner(c.begin(&ctl).unwrap(), next, |tx| {
        tx.execute(&update(&name), vec![42_i64.into()]).map(|_| ())
    })
    .unwrap();
    release_owner(other.begin(&ctl).unwrap(), next).unwrap();
    assert_eq!(
        claim_owner(other.begin(&ctl).unwrap(), token(3))
            .unwrap()
            .generation(),
        3
    );
    assert_eq!(value(&name), 42);
    assert!(!format!("{c:?} {ctl:?}").contains(&name));
    println!("Native exact transitions and surviving stale client: passed");
}
fn rollback() {
    let (name, ctl) = table("rollback");
    let mut c = connect();
    let owner = claim_owner(c.begin(&ctl).unwrap(), token(4)).unwrap();
    let invalid = format!("UPDATE `{name}` SET nonexistent_column=1");
    assert_eq!(
        with_owner(c.begin(&ctl).unwrap(), owner, |tx| {
            tx.execute(&update(&name), vec![99_i64.into()])?;
            tx.execute(&invalid, vec![]).map(|_| ())
        }),
        Err(OwnerError::Unavailable)
    );
    assert_eq!(value(&name), 0);
    let mut tx = c.begin(&ctl).unwrap();
    tx.lock_owner().unwrap();
    tx.execute(&update(&name), vec![88_i64.into()]).unwrap();
    assert_eq!(tx.execute(&invalid, vec![]), Err(OwnerError::Unavailable));
    assert_eq!(tx.lock_owner(), Err(OwnerError::Invalid));
    assert_eq!(tx.commit(), Err(OwnerError::Invalid));
    assert_eq!(value(&name), 0);
    {
        let mut tx = c.begin(&ctl).unwrap();
        tx.lock_owner().unwrap();
        tx.execute(&update(&name), vec![77_i64.into()]).unwrap();
    }
    assert_eq!(value(&name), 0);
    with_owner(c.begin(&ctl).unwrap(), owner, |tx| {
        tx.execute(&update(&name), vec![42_i64.into()]).map(|_| ())
    })
    .unwrap();
    assert_eq!(value(&name), 42);
    println!("Native prefix rollback, caught-error poison and Drop: passed");
}
fn malformed() {
    let (name, ctl) = table("invalid");
    let mut c = connect();
    let wrong = ControlTable::new("rom_extras_tests", &name, [19; 32]).unwrap();
    assert_eq!(
        claim_owner(c.begin(&wrong).unwrap(), token(5)),
        Err(OwnerError::Invalid)
    );
    for (sql, error) in [
        ("format_version=2", OwnerError::UnsupportedFormat),
        ("generation=-1", OwnerError::Invalid),
        (
            "generation=0,owner_token=UNHEX(REPEAT('11',32))",
            OwnerError::Invalid,
        ),
        ("store_identity=UNHEX('11')", OwnerError::Invalid),
        ("generation=1,owner_token=UNHEX('00')", OwnerError::Invalid),
        (
            "generation=1,owner_token=UNHEX(REPEAT('00',32))",
            OwnerError::Invalid,
        ),
        (
            "generation=1,owner_token=UNHEX(REPEAT('11',1048576))",
            OwnerError::Invalid,
        ),
        (
            "store_identity=UNHEX(REPEAT('11',1048576))",
            OwnerError::Invalid,
        ),
    ] {
        raw().exec_drop(format!("UPDATE `{name}` SET format_version=1,store_identity=?,generation=0,owner_token=NULL"),(ID.to_vec(),)).unwrap();
        raw()
            .query_drop(format!("UPDATE `{name}` SET {sql}"))
            .unwrap();
        assert_eq!(claim_owner(c.begin(&ctl).unwrap(), token(5)), Err(error));
    }
    raw().exec_drop(format!("UPDATE `{name}` SET format_version=1,store_identity=?,generation=9223372036854775807,owner_token=NULL"),(ID.to_vec(),)).unwrap();
    assert_eq!(
        claim_owner(c.begin(&ctl).unwrap(), token(5)),
        Err(OwnerError::Exhausted)
    );
    let (empty, emptyctl) = table("empty");
    raw().query_drop(format!("DELETE FROM `{empty}`")).unwrap();
    assert_eq!(
        claim_owner(c.begin(&emptyctl).unwrap(), token(5)),
        Err(OwnerError::Uninitialized)
    );
    let (typed, typedctl) = table("type");
    raw()
        .query_drop(format!(
            "ALTER TABLE `{typed}` MODIFY generation VARCHAR(32) NOT NULL"
        ))
        .unwrap();
    assert_eq!(
        claim_owner(c.begin(&typedctl).unwrap(), token(5)),
        Err(OwnerError::Invalid)
    );
    let missing = ControlTable::new("rom_extras_tests", &format!("missing_{name}"), ID).unwrap();
    assert_eq!(
        claim_owner(c.begin(&missing).unwrap(), token(5)),
        Err(OwnerError::Uninitialized)
    );
    let dup = format!("dup_{name}");
    raw()
        .query_drop(format!(
            "CREATE TABLE `{dup}` ENGINE=InnoDB AS SELECT * FROM `{name}`"
        ))
        .unwrap();
    raw()
        .query_drop(format!("INSERT INTO `{dup}` SELECT * FROM `{name}`"))
        .unwrap();
    let dup = ControlTable::new("rom_extras_tests", &dup, ID).unwrap();
    assert_eq!(
        claim_owner(c.begin(&dup).unwrap(), token(5)),
        Err(OwnerError::Invalid)
    );
    let (non, nonctl) = table("engine");
    raw()
        .query_drop(format!("ALTER TABLE `{non}` ENGINE=MyISAM"))
        .unwrap();
    assert_eq!(
        claim_owner(c.begin(&nonctl).unwrap(), token(5)),
        Err(OwnerError::Invalid)
    );
    assert_eq!(value(&non), 0);
    println!(
        "Native independent malformed fields, missing/duplicate rows, nontransactional engine and exhaustion: passed"
    );
}
fn race() {
    let (name, _) = table("race");
    let (ready, prepared) = mpsc::channel();
    let mut workers = vec![];
    let mut releases = vec![];
    for b in [6, 7] {
        let n = name.clone();
        let ready = ready.clone();
        let (go, released) = mpsc::sync_channel(1);
        releases.push(go);
        workers.push(thread::spawn(move || {
            let ctl = ControlTable::new("rom_extras_tests", &n, ID).unwrap();
            let mut c = connect();
            ready.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(3)).unwrap();
            claim_owner(c.begin(&ctl).unwrap(), token(b))
        }));
    }
    for _ in 0..2 {
        prepared.recv_timeout(Duration::from_secs(3)).unwrap();
    }
    for r in releases {
        r.send(()).unwrap();
    }
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(OwnerError::Busy)))
            .count(),
        1
    );
    println!("Native competing claims: passed");
}
fn ordered() {
    let (name, ctl) = table("order");
    let mut c = connect();
    let owner = claim_owner(c.begin(&ctl).unwrap(), token(13)).unwrap();
    let mut held = c.begin(&ctl).unwrap();
    held.lock_owner().unwrap();
    held.execute(&update(&name), vec![17_i64.into()]).unwrap();
    let n = name.clone();
    let waiter = thread::spawn(move || {
        let ctl = ControlTable::new("rom_extras_tests", &n, ID).unwrap();
        let mut c = Connection::connect_loopback(
            config(),
            Deadlines::new(
                Duration::from_secs(3),
                Duration::from_secs(4),
                Duration::from_secs(2),
            )
            .unwrap(),
        )
        .unwrap();
        takeover_owner(c.begin(&ctl).unwrap(), owner.state(), token(14))
    });
    let end = Instant::now() + Duration::from_millis(1500);
    let mut monitor = monitor();
    loop {
        let waiting:u64=monitor.exec_first("SELECT COUNT(*) FROM information_schema.INNODB_TRX WHERE TRX_STATE='LOCK WAIT' AND TRX_QUERY LIKE ?",(format!("%FROM `rom_extras_tests`.`{name}`%"),)).unwrap().unwrap();
        if waiting == 1 {
            break;
        }
        if Instant::now() >= end {
            let stats: Vec<(String, u64)> = monitor
                .exec(
                    "SELECT TRX_STATE,TRX_QUERY LIKE ? FROM information_schema.INNODB_TRX",
                    (format!("%{name}%"),),
                )
                .unwrap();
            panic!(
                "native exact control waiter not observed; bounded state/match flags: {stats:?}"
            );
        }
        // Native INFORMATION_SCHEMA transaction snapshots are cached; do not poll inside their refresh window.
        thread::sleep(Duration::from_millis(150));
    }
    held.commit().unwrap();
    let next = waiter.join().unwrap().unwrap();
    assert_eq!(next.generation(), 2);
    assert_eq!(value(&name), 17);
    assert_eq!(
        release_owner(c.begin(&ctl).unwrap(), owner),
        Err(OwnerError::Stale)
    );
    println!("Native observed control lock and ordered takeover: passed");
}
fn lock_timeout() {
    let (name, ctl) = table("lock");
    let mut c = connect();
    let owner = claim_owner(c.begin(&ctl).unwrap(), token(8)).unwrap();
    let mut held = c.begin(&ctl).unwrap();
    held.lock_owner().unwrap();
    let mut other = connect();
    let start = Instant::now();
    assert_eq!(
        with_owner(other.begin(&ctl).unwrap(), owner, |tx| tx
            .execute(&update(&name), vec![99_i64.into()])
            .map(|_| ())),
        Err(OwnerError::Unavailable)
    );
    assert!(start.elapsed() >= Duration::from_millis(900));
    held.rollback().unwrap();
    assert_eq!(value(&name), 0);
    with_owner(other.begin(&ctl).unwrap(), owner, |tx| {
        tx.execute(&update(&name), vec![42_i64.into()]).map(|_| ())
    })
    .unwrap();
    assert_eq!(value(&name), 42);
    println!("Native lock timeout and reuse after confirmed rollback: passed");
}
fn transport(dribble: bool) {
    let (name, ctl) = table("wire");
    let mut direct = connect();
    let owner = claim_owner(direct.begin(&ctl).unwrap(), token(9)).unwrap();
    drop(direct);
    let relay = crate::wire_proxy::Relay::new(port());
    let mut c = Connection::connect_loopback(
        proxy_config(relay.port),
        Deadlines::new(
            Duration::from_secs(3),
            Duration::from_secs(2),
            Duration::from_secs(1),
        )
        .unwrap(),
    )
    .unwrap();
    let (go, ready) = mpsc::sync_channel(1);
    let n = name.clone();
    let observer = thread::spawn(move || {
        ready.recv_timeout(Duration::from_secs(3)).unwrap();
        let end = Instant::now() + Duration::from_millis(1500);
        loop {
            let v = value(&n);
            if v == 17 {
                break (v, Instant::now());
            }
            assert!(
                Instant::now() < end,
                "native committed write not independently observed before deadline"
            );
            thread::sleep(Duration::from_millis(5));
        }
    });
    let result = with_owner(c.begin(&ctl).unwrap(), owner, |tx| {
        tx.execute(&update(&name), vec![17_i64.into()])?;
        if dribble {
            relay.arm_delivery(false, true);
        } else {
            relay.arm(true);
        }
        go.send(()).unwrap();
        Ok(())
    });
    let returned = Instant::now();
    assert_eq!(result, Err(OwnerError::Unknown));
    assert!(!c.is_available());
    assert_eq!(c.begin(&ctl).unwrap_err(), OwnerError::Unavailable);
    let (v, t) = observer.join().unwrap();
    assert_eq!(v, 17);
    assert!(t < returned);
    let counts = relay.finish_after_client_close();
    assert!(counts.requests_after_arm > 0 && counts.responses_after_arm > 0);
    if dribble {
        assert!(
            counts.delivered_after_arm > 1
                && counts.delivered_after_arm < counts.responses_after_arm
        );
        assert_eq!(counts.suppressed, 0);
    } else {
        assert!(counts.suppressed > 0);
        assert_eq!(counts.delivered_after_arm, 0);
        assert_eq!(counts.suppressed, counts.responses_after_arm);
    }
    println!("Native committed acknowledgement loss and peer-close retirement: {counts:?}");
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
    let relay = crate::wire_proxy::Relay::new(port());
    let c = Connection::connect_loopback(proxy_config(relay.port), limits()).unwrap();
    rt.block_on(async { drop(c) });
    assert!(relay.finish_after_client_close().client_closed);
    println!("Active Tokio refusal and native peer close after async handoff Drop: passed");
}
