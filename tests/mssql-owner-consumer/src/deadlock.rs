//! Actual native victim evidence, not interpretation of the driver's finite error.
use crate::fixture::*;
use rom_mssql::{Connection, Deadlines};
use rom_sql_core::{OwnerError, OwnerToken, claim_owner, takeover_owner, with_owner};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub fn run() {
    let (name, control) = table("deadlock");
    let (aux, _) = table("cycle");
    let events = format!("xe_{name}");
    sql(&format!(
        "CREATE EVENT SESSION [{events}] ON SERVER ADD EVENT sqlserver.xml_deadlock_report, ADD EVENT sqlserver.error_reported(ACTION(sqlserver.session_id) WHERE(error_number=1205)) ADD TARGET package0.ring_buffer(SET max_memory=512) WITH(MAX_MEMORY=512 KB, MAX_DISPATCH_LATENCY=1 SECONDS); ALTER EVENT SESSION [{events}] ON SERVER STATE=START"
    ));
    let mut driver = Connection::connect(
        config(),
        Deadlines::new(WAIT, Duration::from_secs(20), Duration::from_secs(15)).unwrap(),
    )
    .unwrap();
    let old = claim_owner(
        driver.begin(&control).unwrap(),
        OwnerToken::new([41; 32]).unwrap(),
    )
    .unwrap();
    let mut aggressor = FixtureSession::new();
    aggressor.sql(&format!("SET XACT_ABORT ON; SET DEADLOCK_PRIORITY HIGH; BEGIN TRANSACTION; UPDATE dbo.[{aux}] SET fixture_value=88 WHERE singleton_key=1"));
    let (done_tx, done_rx) = mpsc::channel();
    let ctl = control.clone();
    let primary = name.clone();
    let secondary = aux.clone();
    let join = thread::spawn(move || {
        let result = with_owner(driver.begin(&ctl).unwrap(), old, |tx| {
            tx.execute(&update(&primary), &[&99_i64])?;
            tx.execute(&update(&secondary), &[&99_i64])
        });
        done_tx.send((driver, result)).unwrap();
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let victim = loop {
        let session = scalar(&format!(
            "SELECT CAST(COALESCE(MAX(r.session_id),0) AS BIGINT) FROM sys.dm_exec_requests r CROSS APPLY sys.dm_exec_sql_text(r.sql_handle) t WHERE r.database_id=DB_ID() AND r.blocking_session_id>0 AND r.wait_type LIKE 'LCK_M_%' AND CHARINDEX('UPDATE dbo.[{aux}] SET fixture_value',t.text)>0"
        ));
        if session > 0 {
            break session;
        }
        assert!(
            Instant::now() < deadline,
            "victim did not enter actual native wait"
        );
        thread::sleep(Duration::from_millis(10));
    };
    // This request closes the C/S cycle. HIGH priority makes the driver NORMAL victim.
    aggressor.sql(&format!(
        "UPDATE dbo.[{name}] SET fixture_value=77 WHERE singleton_key=1; ROLLBACK TRANSACTION"
    ));
    let (mut driver, result) = done_rx.recv_timeout(Duration::from_secs(25)).unwrap();
    join.join().unwrap();
    assert_eq!(result, Err(OwnerError::Unavailable));
    assert_eq!(value(&name), 0);
    assert_eq!(value(&aux), 0);
    let error_query = format!(
        "SELECT CAST(COUNT(*) AS BIGINT) FROM (SELECT CAST(t.target_data AS XML) x FROM sys.dm_xe_session_targets t JOIN sys.dm_xe_sessions s ON s.address=t.event_session_address WHERE s.name='{events}' AND t.target_name='ring_buffer') b CROSS APPLY b.x.nodes('/RingBufferTarget/event[@name=\"error_reported\"]') n(e) WHERE n.e.value('(data[@name=\"error_number\"]/value)[1]','int')=1205 AND n.e.value('(action[@name=\"session_id\"]/value)[1]','int')={victim}"
    );
    let graph_query = format!(
        "SELECT CAST(COUNT(*) AS BIGINT) FROM (SELECT CAST(t.target_data AS XML) x FROM sys.dm_xe_session_targets t JOIN sys.dm_xe_sessions s ON s.address=t.event_session_address WHERE s.name='{events}' AND t.target_name='ring_buffer') b CROSS APPLY b.x.nodes('/RingBufferTarget/event[@name=\"xml_deadlock_report\"]/data/value/deadlock') n(d) CROSS APPLY n.d.nodes('process-list/process') p(v) WHERE p.v.value('@spid','int')={victim} AND p.v.value('@id','varchar(128)')=n.d.value('(victim-list/victimProcess/@id)[1]','varchar(128)') AND n.d.exist('resource-list/*/owner-list/owner')=1 AND n.d.exist('resource-list/*/waiter-list/waiter')=1 AND n.d.exist('resource-list/keylock[@objectname=\"rom_extras_tests.dbo.{name}\"]')=1 AND n.d.exist('resource-list/keylock[@objectname=\"rom_extras_tests.dbo.{aux}\"]')=1"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if scalar(&error_query) > 0 && scalar(&graph_query) > 0 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "missing native1205 and matching deadlock victim evidence"
        );
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        scalar(&error_query.replace(&format!("={victim}"), "=-1")),
        0,
        "foreign session cannot qualify native1205"
    );
    assert_eq!(
        scalar(&graph_query.replace(&format!("={victim}"), "=-1")),
        0,
        "foreign session cannot qualify victim graph"
    );
    assert_eq!(
        scalar(&graph_query.replace(
            &format!("rom_extras_tests.dbo.{name}"),
            "rom_extras_tests.dbo.not_this_control_table"
        )),
        0,
        "foreign control table cannot qualify the cycle"
    );
    let payload = sql(&format!(
        "SELECT CAST(t.target_data AS NVARCHAR(MAX)) FROM sys.dm_xe_session_targets t JOIN sys.dm_xe_sessions s ON s.address=t.event_session_address WHERE s.name='{events}' AND t.target_name='ring_buffer'"
    ));
    let xml = payload[0][0].get::<&str, _>(0).unwrap();
    assert!(xml.len() <= 1_048_576, "bounded native event evidence");
    let directory = std::path::Path::new(".superpowers/mssql-owner-native-events");
    std::fs::create_dir_all(directory).unwrap();
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut evidence = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.join(format!("{events}.xml")))
        .unwrap();
    use std::io::Write;
    evidence.write_all(xml.as_bytes()).unwrap();
    println!(
        "Native event evidence retained: {}",
        directory.join(format!("{events}.xml")).display()
    );
    let mut successor = connect();
    let new = takeover_owner(
        successor.begin(&control).unwrap(),
        old.state(),
        OwnerToken::new([42; 32]).unwrap(),
    )
    .unwrap();
    assert_eq!(
        with_owner(driver.begin(&control).unwrap(), old, |tx| tx
            .execute(&update(&name), &[&99_i64])),
        Err(OwnerError::Stale)
    );
    with_owner(driver.begin(&control).unwrap(), new, |tx| {
        tx.execute(&update(&name), &[&42_i64])
    })
    .unwrap();
    assert_eq!(value(&name), 42);
    // Preserve the event session and native tables. Stop event capture after this profile.
    sql(&format!(
        "ALTER EVENT SESSION [{events}] ON SERVER STATE=STOP"
    ));
    println!(
        "Native1205 matched actual victim; prefix rollback, stale retry and current owner passed"
    );
}
