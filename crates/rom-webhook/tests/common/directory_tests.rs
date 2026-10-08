#[test]
fn simultaneous_receivers_with_equal_clock_stamps_have_private_databases() {
    // Fixed clock input makes the formerly probabilistic collision observable.
    let (left, right) = std::thread::scope(|scope| {
        let left = scope.spawn(|| super::reserve(1234));
        let right = scope.spawn(|| super::reserve(1234));
        (left.join().unwrap(), right.join().unwrap())
    });
    assert_ne!(
        left, right,
        "independent receiver databases must not share a directory"
    );
    std::fs::write(left.join("isolation-marker"), b"left").unwrap();
    assert!(!right.join("isolation-marker").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(left).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(right).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}
