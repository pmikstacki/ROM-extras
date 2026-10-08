//! Bounded operations on this isolated fixture only.
pub(crate) fn container(action: &str) -> bool {
    assert!(matches!(action, "pause" | "unpause" | "stop" | "start"));
    let mut command = std::process::Command::new("timeout");
    command.args(["--kill-after=2s", "20s", "docker", action]);
    if action == "stop" {
        command.args(["--time", "10"]);
    }
    command
        .arg("rom-extras-openbao-20261008")
        .output()
        .is_ok_and(|output| output.status.success())
}
pub(crate) struct Paused(bool);
impl Paused {
    pub(crate) fn new() -> Self {
        assert!(container("pause"), "fixture pause failed");
        Self(true)
    }
    pub(crate) fn resume(mut self) {
        let restored = container("unpause");
        self.0 = !restored;
        assert!(restored, "fixture unpause failed");
    }
}
impl Drop for Paused {
    fn drop(&mut self) {
        if self.0 {
            let _ = container("unpause");
        }
    }
}
