//! Native COMMIT response loss through public maintained API, with independent durable observation.
use crate::{fixture::*, wire_proxy::Relay};
use rom_mssql::{Connection, Deadlines};
use rom_sql_core::{
    OwnerError, OwnerToken, OwnerTransaction, claim_owner, takeover_owner, with_owner,
};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub fn run() {
    committed_response_loss(false);
    committed_response_loss(true);
    precommit_disconnect();
}
fn committed_response_loss(suppress: bool) {
    let (name, control) = table("wire");
    let mut direct = connect();
    let owner = claim_owner(
        direct.begin(&control).unwrap(),
        OwnerToken::new([71; 32]).unwrap(),
    )
    .unwrap();
    drop(direct);
    let relay = Relay::new(55440);
    let mut cfg = config();
    cfg.port(relay.port);
    let mut client = Connection::connect(
        cfg,
        Deadlines::new(WAIT, Duration::from_secs(3), Duration::from_millis(500)).unwrap(),
    )
    .unwrap();
    let (start, ready) = mpsc::sync_channel(1);
    let observed_name = name.clone();
    let observer = thread::spawn(move || {
        ready.recv_timeout(WAIT).unwrap();
        // Committed locking read, never NOLOCK. It must wait for the staged UPDATE's X lock.
        let value = scalar(&format!(
            "SELECT fixture_value FROM dbo.[{observed_name}] WITH (READCOMMITTEDLOCK) WHERE singleton_key=1"
        ));
        (value, Instant::now())
    });
    let result = with_owner(client.begin(&control).unwrap(), owner, |tx| {
        tx.execute(
            &format!("UPDATE dbo.[{name}] SET fixture_value=fixture_value+1 WHERE singleton_key=1"),
            &[],
        )?;
        relay.arm(suppress);
        start.send(()).unwrap();
        Ok(())
    });
    let returned = Instant::now();
    if suppress {
        assert_eq!(
            result,
            Err(OwnerError::Unknown),
            "actual lost COMMIT acknowledgement"
        );
        assert!(!client.is_available(), "uncertain native client retired");
        assert_eq!(client.begin(&control).unwrap_err(), OwnerError::Unavailable);
    } else {
        assert_eq!(result, Ok(()), "transparent negative control");
        assert!(client.is_available());
    }
    let (committed, observed) = observer.join().unwrap();
    assert_eq!(committed, 1);
    if suppress {
        assert!(
            observed < returned,
            "actual native commit observed before caller timeout"
        );
    }
    let counts = if suppress {
        let counts = relay.finish_after_client_close();
        drop(client);
        counts
    } else {
        drop(client);
        relay.finish()
    };
    assert!(counts.requests_after_arm > 0);
    assert!(counts.responses_after_arm > 0);
    if suppress {
        assert_eq!(counts.suppressed, counts.responses_after_arm);
        assert_eq!(counts.delivered_after_arm, 0);
    } else {
        assert_eq!(counts.suppressed, 0);
        assert_eq!(counts.delivered_after_arm, counts.responses_after_arm);
    }
    let mut fresh = connect();
    let mut inspection = fresh.begin(&control).unwrap();
    assert_eq!(inspection.lock_owner().unwrap(), owner.state());
    inspection.rollback().unwrap();
    assert_eq!(value(&name), 1, "no automatic persistence retry");
    let successor = takeover_owner(
        fresh.begin(&control).unwrap(),
        owner.state(),
        OwnerToken::new([72; 32]).unwrap(),
    )
    .unwrap();
    assert_eq!(
        with_owner(fresh.begin(&control).unwrap(), owner, |_| Ok(())),
        Err(OwnerError::Stale)
    );
    with_owner(fresh.begin(&control).unwrap(), successor, |tx| {
        tx.execute(&update(&name), &[&42_i64]).map(|_| ())
    })
    .unwrap();
    assert_eq!(value(&name), 42);
    println!(
        "Native COMMIT suppress={suppress}: committed value1, exact owner, takeover/stale fence; ciphertext counts {counts:?}"
    );
}

fn precommit_disconnect() {
    let (name, control) = table("wire_before");
    let mut direct = connect();
    let owner = claim_owner(
        direct.begin(&control).unwrap(),
        OwnerToken::new([73; 32]).unwrap(),
    )
    .unwrap();
    drop(direct);
    let mut relay = Some(Relay::new(55440));
    let mut cfg = config();
    cfg.port(relay.as_ref().unwrap().port);
    let mut client = Connection::connect(
        cfg,
        Deadlines::new(WAIT, Duration::from_secs(3), Duration::from_millis(500)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        with_owner(client.begin(&control).unwrap(), owner, |tx| {
            tx.execute(&update(&name), &[&99_i64])?;
            let _ = relay.take().unwrap().finish(); // Owned sockets close BEFORE native COMMIT.
            Ok(())
        }),
        Err(OwnerError::Unknown)
    );
    assert!(!client.is_available());
    assert_eq!(
        value(&name),
        0,
        "before-COMMIT disconnect is rollback, not committed response loss"
    );
    println!("Native pre-COMMIT disconnect counterexample: Unknown but value0");
}
