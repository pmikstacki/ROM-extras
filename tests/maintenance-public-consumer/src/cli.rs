use std::{path::Path, process::Command};
pub fn run(root: &Path) {
    let binary =
        std::env::var_os("ROM_EXTRAS_MAINTENANCE_BIN").expect("explicit CLI executable required");
    for backend in ["sqlite", "redb"] {
        let archive = root.join(format!("{backend}-archive.rombk"));
        let inspected = Command::new(&binary)
            .args(["inspect", backend])
            .arg(&archive)
            .output()
            .unwrap();
        assert!(inspected.status.success());
        assert!(inspected.stderr.is_empty());
        let body = String::from_utf8(inspected.stdout).unwrap();
        assert!(body.contains("\"rows\":2"));
        assert!(
            !body.contains("private")
                && !body.contains("amount")
                && !body.contains("source_provenance")
        );
        let target = root.join(format!("{backend}-cli-restored.db"));
        let restored = Command::new(&binary)
            .args(["restore", backend])
            .arg(&archive)
            .arg(&target)
            .output()
            .unwrap();
        assert!(restored.status.success());
        assert!(target.is_file());
        assert!(
            String::from_utf8(restored.stdout)
                .unwrap()
                .contains("\"host_cutover_required\":true")
        );
        let repeated = Command::new(&binary)
            .args(["restore", backend])
            .arg(&archive)
            .arg(&target)
            .output()
            .unwrap();
        assert_eq!(repeated.status.code(), Some(2));
        assert_eq!(repeated.stderr, b"maintenance:Conflict\n");
        let limited = Command::new(&binary)
            .args(["inspect", backend])
            .arg(&archive)
            .args(["--max-bytes", "1"])
            .output()
            .unwrap();
        assert_eq!(limited.status.code(), Some(2));
        assert_eq!(limited.stderr, b"maintenance:TooLarge\n");
    }
    let private = root.join("private-path-canary");
    let invalid = Command::new(&binary)
        .args(["inspect", "sql-server"])
        .arg(&private)
        .output()
        .unwrap();
    assert_eq!(invalid.stderr, b"maintenance:Unsupported\n");
    let missing = Command::new(&binary)
        .args(["restore", "sqlite"])
        .arg(&private)
        .arg(root.join("private-destination-canary"))
        .output()
        .unwrap();
    assert_eq!(missing.stderr, b"maintenance:Unavailable\n");
    assert!(
        !String::from_utf8(missing.stderr)
            .unwrap()
            .contains("canary")
    );
    use std::os::unix::ffi::OsStringExt;
    let invalid_path = std::ffi::OsString::from_vec(b"private-path-canary-\xff".to_vec());
    let result = Command::new(&binary)
        .args(["inspect", "sqlite"])
        .arg(invalid_path)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(result.stderr, b"maintenance:Invalid\n");
    println!(
        "CLI: actual inspect/fresh restore, count-only output, repeated destination/limit/unsupported/private-path/non-UTF8 negatives passed"
    );
}
