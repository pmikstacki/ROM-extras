use crate::{ControlTable, Deadlines};
use std::time::Duration;
#[test]
fn configuration_rejects_injection_unbounded_and_fractional_deadlines() {
    for value in ["", "dbo.table", "bad]", "1table", "ż", "x;DROP"] {
        assert!(ControlTable::new("dbo", value, [17; 32]).is_err());
    }
    assert!(ControlTable::new("dbo", &"x".repeat(64), [17; 32]).is_err());
    assert!(ControlTable::new("dbo", &"x".repeat(63), [17; 32]).is_ok());
    let second = Duration::from_secs(1);
    for (connect, io, lock) in [
        (Duration::ZERO, second, second),
        (second, Duration::ZERO, second),
        (second, second, second),
        (second, second, Duration::ZERO),
        (second, second, Duration::from_nanos(1)),
        (Duration::from_secs(61), second, Duration::from_millis(1)),
    ] {
        assert!(Deadlines::new(connect, io, lock).is_err());
    }
    assert!(Deadlines::new(second, second, Duration::from_millis(1)).is_ok());
    assert_eq!(
        format!(
            "{:?}",
            ControlTable::new("dbo", "secret_host_table", [17; 32]).unwrap()
        ),
        "ControlTable { redacted }"
    );
}
