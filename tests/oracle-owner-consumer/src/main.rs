//! Independent public Oracle ownership consumer over the explicitly controlled native fixture.
mod cases;
mod fixture;
mod restart;
use rom_oracle::Value;
use rom_sql_core::{
    OwnerError, OwnerTransaction, claim_owner, release_owner, takeover_owner, with_owner,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--restart" {
        restart::run(std::path::Path::new(&args[2]));
        return;
    }
    assert_eq!(args.len(), 1, "unknown qualification mode");
    let f = fixture::Fixture::new();
    let mut first = fixture::connection();
    let mut old = fixture::connection();
    let g1 = claim_owner(first.begin(&f.control).unwrap(), fixture::token(1)).unwrap();
    assert_eq!(g1.generation(), 1);
    assert_eq!(
        claim_owner(old.begin(&f.control).unwrap(), fixture::token(2)),
        Err(OwnerError::Busy)
    );
    let observed = old.begin(&f.control).unwrap().lock_owner().unwrap();
    let g2 = takeover_owner(
        first.begin(&f.control).unwrap(),
        observed,
        fixture::token(2),
    )
    .unwrap();
    assert_eq!(g2.generation(), 2);
    assert_eq!(
        with_owner(old.begin(&f.control).unwrap(), g1, |_| Ok(())),
        Err(OwnerError::Stale)
    );
    with_owner(first.begin(&f.control).unwrap(), g2, |tx| {
        assert_eq!(tx.execute(&f.update(), &[Value::Integer(Some(17))])?, 1);
        Ok(())
    })
    .unwrap();
    assert_eq!(f.value(), 17);
    release_owner(first.begin(&f.control).unwrap(), g2).unwrap();
    assert_eq!(
        release_owner(old.begin(&f.control).unwrap(), g1),
        Err(OwnerError::Stale)
    );
    println!("Oracle public ownership transitions passed");
    cases::run();
}
