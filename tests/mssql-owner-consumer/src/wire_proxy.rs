//! Test-only opaque relay. Never terminates TLS or records payload/credentials.
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
const LIMIT: usize = 8 * 1024 * 1024;
const TICK: Duration = Duration::from_millis(5);
#[derive(Default, Debug)]
pub struct Counts {
    pub client_closed: bool,
    pub requests_after_arm: usize,
    pub responses_after_arm: usize,
    pub suppressed: usize,
    pub delivered_after_arm: usize,
}
struct Arm {
    suppress: bool,
    dribble: bool,
    ack: SyncSender<()>,
}
pub struct Relay {
    pub port: u16,
    commands: Sender<Arm>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Counts>>,
}
impl Relay {
    /// Shared qualification relay for the fixed retained native fixtures only.
    pub fn new(backend_port: u16) -> Self {
        assert!(matches!(
            backend_port,
            55439 | 55440 | 55452 | 55453 | 55457
        ));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let (commands, receiver) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || pump(listener, receiver, worker_stop, backend_port));
        Self {
            port,
            commands,
            stop,
            worker: Some(worker),
        }
    }
    // One pump owns BOTH directions: acknowledgement follows any prior write_all.
    // No response delivery can race this barrier; the next caller operation is COMMIT.
    pub fn arm(&self, suppress: bool) {
        self.arm_delivery(suppress, false);
    }
    /// Arm one delivery barrier. Pacing sends real native bytes at one byte per250ms.
    /// Suppression and pacing are mutually exclusive; no synthetic response is created.
    pub fn arm_delivery(&self, suppress: bool, dribble: bool) {
        assert!(!(suppress && dribble));
        let (ack, ready) = mpsc::sync_channel(1);
        self.commands
            .send(Arm {
                suppress,
                dribble,
                ack,
            })
            .unwrap();
        ready
            .recv_timeout(Duration::from_secs(2))
            .expect("delivery barrier");
    }
    pub fn finish(mut self) -> Counts {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap()
    }
    /// Confirm native retirement closed the accepted peer before this helper closes any socket.
    pub fn finish_after_client_close(mut self) -> Counts {
        let deadline = Instant::now() + Duration::from_secs(1);
        while !self.worker.as_ref().unwrap().is_finished() {
            assert!(
                Instant::now() < deadline,
                "retired native peer remained open"
            );
            thread::sleep(TICK);
        }
        let counts = self.worker.take().unwrap().join().unwrap();
        assert!(
            counts.client_closed,
            "server EOF cannot establish native peer retirement"
        );
        counts
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn pump(
    listener: TcpListener,
    commands: Receiver<Arm>,
    stop: Arc<AtomicBool>,
    backend_port: u16,
) -> Counts {
    let deadline = Instant::now() + Duration::from_secs(25);
    let mut counts = Counts::default();
    let mut client = loop {
        if stop.load(Ordering::SeqCst) {
            return counts;
        }
        assert!(Instant::now() < deadline, "relay accept deadline");
        match listener.accept() {
            Ok((stream, address)) => {
                assert!(address.ip().is_loopback());
                break stream;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(TICK),
            Err(_) => panic!("relay accept failed"),
        }
    };
    let address: SocketAddr = ([127, 0, 0, 1], backend_port).into();
    let mut server = TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
    for stream in [&client, &server] {
        stream.set_nodelay(true).unwrap();
        stream.set_read_timeout(Some(TICK)).unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(1)))
            .unwrap();
    }
    let mut buffer = [0_u8; 16 * 1024];
    let mut total = 0_usize;
    let mut armed = None;
    let mut dribble = false;
    let mut pending = std::collections::VecDeque::new();
    let mut next_delivery = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "relay lifetime deadline");
        if let Ok(arm) = commands.try_recv() {
            assert!(armed.is_none(), "one arm per connection");
            armed = Some(arm.suppress);
            dribble = arm.dribble;
            next_delivery = Instant::now() + Duration::from_millis(250);
            arm.ack.send(()).unwrap();
        }
        let Some(n) = read(&mut client, &mut buffer) else {
            counts.client_closed = true;
            break;
        };
        total = total.checked_add(n).unwrap();
        assert!(total <= LIMIT, "relay byte limit");
        if n > 0 {
            server.write_all(&buffer[..n]).expect("request forwarding");
            if armed.is_some() {
                counts.requests_after_arm += n;
            }
        }
        if !pending.is_empty() {
            if Instant::now() >= next_delivery {
                let byte = pending.pop_front().unwrap();
                client.write_all(&[byte]).expect("paced native response");
                counts.delivered_after_arm += 1;
                next_delivery = Instant::now() + Duration::from_millis(250);
            }
            continue;
        }
        let Some(n) = read(&mut server, &mut buffer) else {
            break;
        };
        total = total.checked_add(n).unwrap();
        assert!(total <= LIMIT, "relay byte limit");
        if n > 0 {
            if let Some(suppress) = armed {
                counts.responses_after_arm += n;
                if suppress {
                    counts.suppressed += n;
                    continue;
                }
                if dribble {
                    pending.extend(&buffer[..n]);
                    continue;
                }
                counts.delivered_after_arm += n;
            }
            client.write_all(&buffer[..n]).expect("response forwarding");
        }
    }
    counts
}
fn read(stream: &mut TcpStream, buffer: &mut [u8]) -> Option<usize> {
    match stream.read(buffer) {
        Ok(0) => None,
        Ok(n) => Some(n),
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            Some(0)
        }
        // A retired driver may reset its owned socket; no bytes are inferred from EOF.
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) =>
        {
            None
        }
        Err(_) => panic!("relay read failed"),
    }
}
