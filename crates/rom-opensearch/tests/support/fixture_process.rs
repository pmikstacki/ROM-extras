//! Owned finite fixture startup; native test processes and their readiness reader are joined on drop.
use std::{
    io::{self, BufRead, BufReader, Read},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread::JoinHandle,
    time::Duration,
};
pub(crate) struct FixtureProcess {
    child: Child,
    reader: Option<JoinHandle<()>>,
}
impl FixtureProcess {
    pub(crate) fn start(command: &mut Command, deadline: Duration) -> io::Result<(Self, u16)> {
        let mut owner = Self {
            child: command
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?,
            reader: None,
        };
        let stdout = owner
            .child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("missing readiness pipe"))?;
        let (send, receive) = mpsc::sync_channel(1);
        owner.reader = Some(std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout)
                .take(16)
                .read_line(&mut line)
                .map(|_| line);
            let _ = send.send(result);
        }));
        let line = receive
            .recv_timeout(deadline)
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "fixture readiness timeout"))??;
        let port = line
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|p| *p != 0)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid fixture readiness")
            })?;
        Ok((owner, port))
    }
}
impl Drop for FixtureProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
