use crate::fixture::{self, Fixture};
use rom_oracle::{Config, Connection, ControlTable, TlsWallet, Value};
use rom_sql_core::{OwnerError, OwnerState, OwnerTransaction, claim_owner, with_owner};
use std::time::{Duration, Instant};

pub(crate) fn run() {
    let f = Fixture::new();
    let mut c = fixture::connection();
    let owner = claim_owner(c.begin(&f.control).unwrap(), fixture::token(3)).unwrap();
    let update = f.update();
    // Oracle rolls back only the failed duplicate statement; the maintained wrapper must roll back the prefix.
    assert_eq!(
        with_owner(c.begin(&f.control).unwrap(), owner, |tx| {
            tx.execute(&update, &[Value::Integer(Some(99))])?;
            tx.execute(&format!("INSERT INTO {} VALUES(1,42)", f.data), &[])?;
            Ok(())
        }),
        Err(OwnerError::Unavailable)
    );
    assert_eq!(f.value(), 0);
    assert!(c.is_available());
    assert_eq!(
        with_owner(c.begin(&f.control).unwrap(), owner, |tx| {
            tx.execute(&update, &[Value::Integer(Some(99))])?;
            assert_eq!(
                tx.execute(&format!("INSERT INTO {} VALUES(1,42)", f.data), &[]),
                Err(OwnerError::Unavailable)
            );
            assert_eq!(
                tx.execute(&update, &[Value::Integer(Some(88))]),
                Err(OwnerError::Invalid)
            );
            Ok(())
        }),
        Err(OwnerError::Invalid)
    );
    assert_eq!(f.value(), 0);
    {
        let mut tx = c.begin(&f.control).unwrap();
        assert_eq!(tx.lock_owner().unwrap(), owner.state());
        tx.execute(&update, &[Value::Integer(Some(66))]).unwrap();
    }
    assert_eq!(f.value(), 0);
    with_owner(c.begin(&f.control).unwrap(), owner, |tx| {
        assert_eq!(tx.execute(&update, &[Value::Integer(Some(17))])?, 1);
        assert_eq!(tx.execute(&update, &[Value::Integer(Some(17))])?, 1);
        Ok(())
    })
    .unwrap();
    assert_eq!(f.value(), 17);
    println!("Oracle native rollback, caught-error poison, Drop and fresh reuse passed");

    let forbidden_table = format!("{}_FORBIDDEN", f.data);
    for sql in [
        "SELECT 1 FROM dual".to_string(),
        "COMMIT".to_string(),
        "ROLLBACK".to_string(),
        "BEGIN NULL; END;".to_string(),
        format!("CREATE TABLE {forbidden_table} (value NUMBER)"),
        format!("UPDATE {} SET value=22 RETURNING value INTO :1", f.data),
    ] {
        assert_eq!(
            with_owner(c.begin(&f.control).unwrap(), owner, |tx| {
                tx.execute(&update, &[Value::Integer(Some(99))])?;
                tx.execute(&sql, &[])?;
                Ok(())
            }),
            Err(OwnerError::Invalid)
        );
        assert_eq!(f.value(), 17);
        assert!(c.is_available());
    }
    assert_eq!(
        fixture::observer()
            .query_row_as::<i64>(
                "SELECT count(*) FROM user_tables WHERE table_name=:1",
                &[&forbidden_table]
            )
            .unwrap(),
        0
    );
    println!(
        "Oracle native forbidden statement kinds rejected before execution and no implicit DDL commit passed"
    );

    for values in [
        vec![Value::Raw(Some(vec![4; 32768]))],
        vec![Value::Raw(Some(vec![4; 32767])); 3],
        vec![Value::Integer(None); 65],
        vec![Value::Double(Some(f64::NAN))],
        vec![Value::Double(Some(f64::INFINITY))],
    ] {
        assert_eq!(
            with_owner(c.begin(&f.control).unwrap(), owner, |tx| {
                tx.execute(&update, &[Value::Integer(Some(99))])?;
                tx.execute(&update, &values)?;
                Ok(())
            }),
            Err(OwnerError::Invalid)
        );
        assert_eq!(f.value(), 17);
    }
    assert_eq!(
        with_owner(c.begin(&f.control).unwrap(), owner, |tx| {
            tx.execute(&" ".repeat(16385), &[])?;
            Ok(())
        }),
        Err(OwnerError::Invalid)
    );
    println!("Oracle native bounded SQL/binds and nonfinite values passed");

    let mut contender = fixture::connection();
    let mut locked = c.begin(&f.control).unwrap();
    assert_eq!(locked.lock_owner().unwrap(), owner.state());
    let start = Instant::now();
    assert_eq!(
        claim_owner(contender.begin(&f.control).unwrap(), fixture::token(4)),
        Err(OwnerError::Unavailable)
    );
    assert!((Duration::from_millis(800)..Duration::from_secs(3)).contains(&start.elapsed()));
    assert!(contender.is_available());
    locked.rollback().unwrap();
    assert_eq!(
        claim_owner(contender.begin(&f.control).unwrap(), fixture::token(4)),
        Err(OwnerError::Busy)
    );
    println!(
        "Oracle native SELECT control lock precedes writes; WAIT timeout and full rollback/reuse passed"
    );

    malformed(&f, &mut c);
    native_call_timeout();
    admission();
    let owner = claim_owner(c.begin(&f.control).unwrap(), fixture::token(9)).unwrap();
    let err = with_owner(c.begin(&f.control).unwrap(), owner, |tx| {
        tx.execute(&f.update(), &[Value::Integer(Some(99))])?;
        tx.execute(
            &format!("UPDATE {} SET secret_error_canary=42", f.data),
            &[],
        )?;
        Ok(())
    })
    .unwrap_err();
    assert_eq!(err, OwnerError::Unknown);
    assert!(!c.is_available());
    assert!(!format!("{err:?}{c:?}").contains("canary"));
    assert_eq!(f.value(), 17);
    assert!(matches!(c.begin(&f.control), Err(OwnerError::Unavailable)));
    println!(
        "Oracle unclassified native failure retires connection, hides diagnostic canary and leaves no prefix write"
    );
}
fn malformed(f: &Fixture, c: &mut Connection) {
    let reset = || {
        let raw = fixture::observer();
        raw.execute(
            &format!(
                "UPDATE {} SET format_version=1,store_identity=:1,generation=0,owner_token=NULL",
                f.owner
            ),
            &[&&[7_u8; 32][..]],
        )
        .unwrap();
        raw.commit().unwrap();
    };
    let inspect = |c: &mut Connection| {
        let mut tx = c.begin(&f.control).unwrap();
        let state = tx.lock_owner();
        tx.rollback().unwrap();
        state
    };
    for (assignment, expected) in [
        ("format_version=2", OwnerError::UnsupportedFormat),
        ("format_version=1.1", OwnerError::UnsupportedFormat),
        ("format_version=NULL", OwnerError::UnsupportedFormat),
        ("store_identity=HEXTORAW('01')", OwnerError::Invalid),
        ("store_identity=NULL", OwnerError::Invalid),
        (
            "store_identity=HEXTORAW(RPAD('02',64,'02'))",
            OwnerError::Invalid,
        ),
        ("generation=-1", OwnerError::Invalid),
        ("generation=1.1", OwnerError::Invalid),
        ("generation=NULL", OwnerError::Invalid),
        ("generation=9223372036854775808", OwnerError::Invalid),
        (
            "generation=0,owner_token=HEXTORAW(RPAD('01',64,'01'))",
            OwnerError::Invalid,
        ),
        (
            "generation=1,owner_token=HEXTORAW(RPAD('00',64,'00'))",
            OwnerError::Invalid,
        ),
        (
            "generation=1,owner_token=HEXTORAW('01')",
            OwnerError::Invalid,
        ),
        (
            "generation=1,owner_token=HEXTORAW(RPAD('02',2000,'02'))",
            OwnerError::Invalid,
        ),
    ] {
        reset();
        assert_eq!(inspect(c).unwrap(), OwnerState::new(0, None).unwrap());
        let raw = fixture::observer();
        raw.execute(&format!("UPDATE {} SET {assignment}", f.owner), &[])
            .unwrap();
        raw.commit().unwrap();
        assert_eq!(inspect(c), Err(expected), "independent malformed case");
        assert!(c.is_available());
    }
    reset();
    let raw = fixture::observer();
    raw.execute(
        &format!("UPDATE {} SET generation=9223372036854775807", f.owner),
        &[],
    )
    .unwrap();
    raw.commit().unwrap();
    assert_eq!(inspect(c).unwrap().generation(), i64::MAX as u64);
    assert_eq!(
        claim_owner(c.begin(&f.control).unwrap(), fixture::token(7)),
        Err(OwnerError::Exhausted)
    );
    assert_eq!(inspect(c).unwrap().generation(), i64::MAX as u64);
    reset();
    let typed = format!("{}_T", f.owner);
    raw.execute(&format!("CREATE TABLE {typed} (singleton_key NUMBER,format_version VARCHAR2(100),store_identity RAW(32),generation NUMBER,owner_token RAW(32))"), &[]).unwrap();
    raw.execute(
        &format!("INSERT INTO {typed} VALUES(1,'1',:1,0,NULL)"),
        &[&&[7_u8; 32][..]],
    )
    .unwrap();
    raw.commit().unwrap();
    let table = ControlTable::new("ROM_EXTRAS", &typed, [7; 32]).unwrap();
    let mut tx = c.begin(&table).unwrap();
    assert_eq!(tx.lock_owner(), Err(OwnerError::Invalid));
    tx.rollback().unwrap();
    // Exercise original character columns separately; CASE projections must not hide coercion.
    for (suffix, format_type, identity_type, generation_type, token_type) in [
        ("CI", "NUMBER", "VARCHAR2(64)", "NUMBER", "RAW(32)"),
        ("CT", "NUMBER", "RAW(32)", "NUMBER", "VARCHAR2(64)"),
        ("CG", "NUMBER", "RAW(32)", "VARCHAR2(64)", "RAW(32)"),
    ] {
        let name = format!("{}_{suffix}", f.owner);
        raw.execute(&format!("CREATE TABLE {name} (singleton_key NUMBER,format_version {format_type},store_identity {identity_type},generation {generation_type},owner_token {token_type})"), &[]).unwrap();
        raw.execute(
            &format!(
                "INSERT INTO {name} VALUES(1,1,{},1,{})",
                if identity_type.starts_with("VARCHAR") {
                    "RPAD('07',64,'07')"
                } else {
                    "HEXTORAW(RPAD('07',64,'07'))"
                },
                if token_type.starts_with("VARCHAR") {
                    "RPAD('03',64,'03')"
                } else {
                    "HEXTORAW(RPAD('03',64,'03'))"
                }
            ),
            &[],
        )
        .unwrap();
        raw.commit().unwrap();
        let mut original = raw
            .statement(&format!(
                "SELECT store_identity,owner_token,generation FROM {name} WHERE 1=0"
            ))
            .fetch_array_size(1)
            .prefetch_rows(0)
            .build()
            .unwrap();
        let description = original.query(&[]).unwrap();
        println!(
            "Oracle character fixture {suffix} native source types: {:?}",
            description
                .column_info()
                .iter()
                .map(|i| i.oracle_type())
                .collect::<Vec<_>>()
        );
        drop(description);
        if suffix == "CI" {
            let result = raw.query_row_as::<Option<Vec<u8>>>(&format!("SELECT CASE WHEN UTL_RAW.LENGTH(store_identity)=32 THEN store_identity ELSE CAST(NULL AS RAW(32)) END FROM {name}"), &[]);
            println!(
                "Oracle character identity CASE native error code: {:?}",
                result.err().and_then(|e| e.oci_code())
            );
        }
        let table = ControlTable::new("ROM_EXTRAS", &name, [7; 32]).unwrap();
        let mut tx = c.begin(&table).unwrap();
        assert_eq!(
            tx.lock_owner(),
            Err(OwnerError::Invalid),
            "native character-column rejection: {suffix}"
        );
        tx.rollback().unwrap();
        assert!(c.is_available());
    }
    let duplicate = format!("{}_X", f.owner);
    raw.execute(
        &format!("CREATE TABLE {duplicate} AS SELECT * FROM {}", f.owner),
        &[],
    )
    .unwrap();
    raw.execute(
        &format!("INSERT INTO {duplicate} SELECT * FROM {}", f.owner),
        &[],
    )
    .unwrap();
    raw.commit().unwrap();
    let table = ControlTable::new("ROM_EXTRAS", &duplicate, [7; 32]).unwrap();
    let mut tx = c.begin(&table).unwrap();
    assert_eq!(tx.lock_owner(), Err(OwnerError::Invalid));
    tx.rollback().unwrap();
    raw.execute(&format!("DELETE FROM {duplicate}"), &[])
        .unwrap();
    raw.commit().unwrap();
    let mut tx = c.begin(&table).unwrap();
    assert_eq!(tx.lock_owner(), Err(OwnerError::Uninitialized));
    tx.rollback().unwrap();
    let missing = ControlTable::new("ROM_EXTRAS", "OWN_MISSING_TABLE_20261010", [7; 32]).unwrap();
    let mut tx = c.begin(&missing).unwrap();
    assert_eq!(tx.lock_owner(), Err(OwnerError::Uninitialized));
    tx.rollback().unwrap();
    println!(
        "Oracle native independent RAW/NUMBER malformations, exact i64::MAX/exhaustion, string coercion, duplicate/empty/missing rows passed"
    );
}
fn admission() {
    for host in ["", "x)(PROTOCOL=TCP", "127.0.0.1\n", "x/y", "0.0.0.0"] {
        assert!(Config::new(host, 55458, "FREEPDB1", "safe", "secret-canary").is_err());
    }
    assert!(
        Config::new(
            "127.0.0.1",
            55458,
            "x)(SERVICE_NAME=y)",
            "safe",
            "secret-canary"
        )
        .is_err()
    );
    let remote = Config::new(
        "example.invalid",
        55458,
        "FREEPDB1",
        "safe",
        "secret-canary",
    )
    .unwrap();
    assert!(!format!("{remote:?}").contains("secret-canary"));
    assert!(matches!(
        Connection::connect_loopback(remote, fixture::bounds()),
        Err(OwnerError::Invalid)
    ));
    assert!(TlsWallet::new("relative/path", "CN=safe").is_err());
    assert!(TlsWallet::new("/tmp/wallet", "CN=safe)(SSL_SERVER_DN_MATCH=NO)").is_err());
    let v = Value::Text(Some("secret-canary".into()));
    assert!(!format!("{v:?}").contains("secret-canary"));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        let cfg = Config::new("127.0.0.1", 55458, "FREEPDB1", "safe", "secret-canary").unwrap();
        assert!(matches!(
            Connection::connect_loopback(cfg, fixture::bounds()),
            Err(OwnerError::Invalid)
        ));
    });
    println!("Oracle structured endpoint, native-runtime admission and safe Debug passed");
}

fn native_call_timeout() {
    let f = Fixture::new();
    let held = fixture::observer();
    held.execute(&format!("INSERT INTO {} VALUES(2,0)", f.data), &[])
        .unwrap();
    held.commit().unwrap();
    let mut c = fixture::connection();
    let owner = claim_owner(c.begin(&f.control).unwrap(), fixture::token(11)).unwrap();
    held.execute(
        &format!("UPDATE {} SET value=0 WHERE singleton_key=1", f.data),
        &[],
    )
    .unwrap();
    let start = Instant::now();
    let result = with_owner(c.begin(&f.control).unwrap(), owner, |tx| {
        tx.execute(
            &format!("UPDATE {} SET value=99 WHERE singleton_key=2", f.data),
            &[],
        )?;
        tx.execute(&f.update(), &[Value::Integer(Some(17))])?;
        Ok(())
    });
    assert_eq!(result, Err(OwnerError::Unknown));
    assert!(!c.is_available());
    assert!(
        (Duration::from_millis(2500)..Duration::from_secs(12)).contains(&start.elapsed()),
        "native controlled roundtrip timeout/cleanup observation"
    );
    held.rollback().unwrap();
    assert_eq!(
        fixture::observer()
            .query_row_as::<i64>(&format!("SELECT SUM(value) FROM {}", f.data), &[])
            .unwrap(),
        0
    );
    println!(
        "Oracle actual OCI call timeout retired native use and removed protected prefix; physical absolute containment remains unqualified"
    );
}
