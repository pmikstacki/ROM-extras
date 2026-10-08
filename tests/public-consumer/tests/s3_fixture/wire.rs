//! Bounded loopback proxy; raw signed requests never leave private memory.
use super::framing;
use rom_blob::ObjectKey;
use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Default)]
pub(crate) struct Counts {
    pub puts: usize,
    pub successes: usize,
    pub conflicts: usize,
    pub gets: usize,
    pub dropped: usize,
    pub key: Option<ObjectKey>,
}
pub(crate) struct Proxy {
    endpoint: String,
    stop: Arc<AtomicBool>,
    counts: Arc<Mutex<Counts>>,
    worker: Option<thread::JoinHandle<Result<(), &'static str>>>,
}
impl Proxy {
    pub(crate) fn start() -> Self {
        // Validate the configured fixed fixture before choosing its socket.
        let _ = super::adapter(1024, false);
        let endpoint = std::env::var("ROM_EXTRAS_S3_ENDPOINT").unwrap();
        let backend: SocketAddr = endpoint.strip_prefix("http://").unwrap().parse().unwrap();
        assert!(backend.ip().is_loopback());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let counts = Arc::new(Mutex::new(Counts::default()));
        let worker = {
            let (stop, counts) = (stop.clone(), counts.clone());
            thread::spawn(move || {
                let mut accepted = 0;
                let deadline = Instant::now() + Duration::from_secs(60);
                while !stop.load(Ordering::SeqCst) {
                    if Instant::now() >= deadline {
                        return Err("proxy lifetime limit");
                    }
                    match listener.accept() {
                        Ok((mut front, peer)) => {
                            if !peer.ip().is_loopback() {
                                return Err("non-loopback peer");
                            }
                            accepted += 1;
                            if accepted > 128 {
                                return Err("connection limit");
                            }
                            forward(&mut front, backend, &counts)?;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5))
                        }
                        Err(_) => return Err("accept failure"),
                    }
                }
                Ok(())
            })
        };
        Self {
            endpoint,
            stop,
            counts,
            worker: Some(worker),
        }
    }
    pub(crate) fn endpoint(&self) -> &str {
        &self.endpoint
    }
    pub(crate) fn counts(&self) -> Counts {
        self.counts.lock().unwrap().clone()
    }
    pub(crate) async fn drain(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let worker = self.worker.take().unwrap();
        tokio::task::spawn_blocking(move || {
            worker
                .join()
                .expect("proxy worker panic")
                .expect("proxy framing failure")
        })
        .await
        .unwrap();
    }
}
impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take()
            && let Ok(Err(category)) = worker.join()
        {
            eprintln!("proxy fixture failure category: {category}");
        }
    }
}
fn forward(
    front: &mut TcpStream,
    backend: SocketAddr,
    counts: &Mutex<Counts>,
) -> Result<(), &'static str> {
    let request = framing::header(front)?;
    let text = framing::text(&request)?;
    let parts: Vec<_> = text
        .lines()
        .next()
        .ok_or("request line")?
        .split_whitespace()
        .collect();
    if parts.len() != 3 || parts[2] != "HTTP/1.1" {
        return Err("unsupported request line");
    }
    let method = parts[0];
    if !matches!(method, "PUT" | "GET" | "HEAD") {
        return Err("unsupported method");
    }
    let key = ObjectKey::parse(
        parts[1]
            .strip_prefix("/rom-extras/")
            .ok_or("unexpected object path")?,
    )
    .map_err(|_| "invalid object key")?;
    let size = framing::length(text, method == "PUT")?;
    let body = framing::body(front, size)?;
    let mut back = TcpStream::connect_timeout(&backend, Duration::from_secs(1))
        .map_err(|_| "backend connect")?;
    // Preserve Host, signed headers, path and body byte-for-byte.
    framing::write(&mut back, &request)?;
    framing::write(&mut back, &body)?;
    {
        let mut c = counts.lock().unwrap();
        if method == "PUT" {
            c.puts += 1;
        }
        if method == "GET" {
            c.gets += 1;
        }
    }
    let mut interim = Vec::new();
    let (response, status) = loop {
        let response = framing::header(&mut back)?;
        let text = framing::text(&response)?;
        let status = text
            .split_whitespace()
            .nth(1)
            .ok_or("response status")?
            .parse::<u16>()
            .map_err(|_| "response status")?;
        if (100..200).contains(&status) {
            if status == 101 || interim.len() + response.len() > 32768 {
                return Err("unsupported interim response");
            }
            interim.extend(response);
            continue;
        }
        break (response, status);
    };
    let response_size = framing::length(
        framing::text(&response)?,
        method != "HEAD" && status != 204 && status != 304,
    )?;
    let response_body = framing::body(&mut back, if method == "HEAD" { 0 } else { response_size })?;
    {
        let mut c = counts.lock().unwrap();
        if method == "PUT" {
            if (200..300).contains(&status) {
                c.successes += 1;
                c.key = Some(key);
            }
            if status == 412 || status == 409 {
                c.conflicts += 1;
            }
        }
    }
    // Suppress all response bytes only after one successful create is fully observed.
    {
        let mut c = counts.lock().unwrap();
        if method == "PUT" && (200..300).contains(&status) && c.dropped == 0 {
            c.dropped = 1;
            return Ok(());
        }
    }
    framing::write(front, &interim)?;
    framing::write(front, &framing::closing_response(&response)?)?;
    framing::write(front, &response_body)
}
