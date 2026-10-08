//! Bounded lifecycle of the dedicated local Oracle fixture.
use std::{
    process::{Command, Output},
    time::{Duration, Instant},
};
const NAME: &str = "rom-extras-oracle-20261008";
fn docker(seconds: &str, args: &[&str]) -> Output {
    Command::new("timeout")
        .arg(seconds)
        .arg("docker")
        .args(args)
        .output()
        .expect("bounded Docker command")
}
pub(crate) fn restart() {
    let label = docker(
        "5",
        &[
            "inspect",
            "--format",
            "{{index .Config.Labels \"rom-extras.fixture\"}}",
            NAME,
        ],
    );
    assert!(label.status.success());
    assert_eq!(String::from_utf8(label.stdout).unwrap().trim(), "oracle");
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let stopped = docker("100", &["stop", "--time", "90", NAME]);
    assert!(stopped.status.success(), "fixture stop failed");
    let state = docker(
        "5",
        &[
            "inspect",
            "--format",
            "{{.State.Status}} {{.State.ExitCode}}",
            NAME,
        ],
    );
    assert!(state.status.success());
    assert_eq!(
        String::from_utf8(state.stdout).unwrap().trim(),
        "exited 143",
        "the pinned Oracle shell returns SIGTERM status after its shutdown handler"
    );
    let logs = docker("5", &["logs", "--since", &since, NAME]);
    assert!(logs.status.success());
    let text = String::from_utf8_lossy(&logs.stdout);
    assert!(
        text.contains("ORACLE instance shut down."),
        "fresh Oracle shutdown confirmation required"
    );
    assert!(docker("30", &["start", NAME]).status.success());
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let ready = docker(
            "5",
            &[
                "exec",
                NAME,
                "/bin/bash",
                "-c",
                "$ORACLE_HOME/bin/sqlplus -s '/ as sysdba' <<'SQL'\nwhenever sqlerror exit failure\nset heading off feedback off\nselect open_mode from v$pdbs where name='FREEPDB1';\nexit\nSQL",
            ],
        );
        if ready.status.success() && String::from_utf8_lossy(&ready.stdout).contains("READ WRITE") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Oracle restart readiness deadline"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}
