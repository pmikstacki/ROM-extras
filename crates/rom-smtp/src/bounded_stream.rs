//! Bound plaintext reads before the protocol parser can allocate an unterminated line.
use lettre::transport::smtp::client::AsyncTokioStream;
use std::{
    fmt, io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};
use tokio_rustls::client::TlsStream;
pub(crate) struct BoundedStream {
    stream: TlsStream<TcpStream>,
    remaining: usize,
    greeting: Vec<u8>,
}
impl BoundedStream {
    pub(crate) fn new(stream: TlsStream<TcpStream>, remaining: usize) -> Self {
        Self {
            stream,
            remaining,
            greeting: Vec::with_capacity(3),
        }
    }
}
impl fmt::Debug for BoundedStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoundedSmtpStream").finish_non_exhaustive()
    }
}
impl AsyncTokioStream for BoundedStream {
    fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.get_ref().0.peer_addr()
    }
}
impl AsyncRead for BoundedStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        if self.remaining == 0 {
            return Poll::Ready(Err(io::Error::other("SMTP reply budget exceeded")));
        }
        let mut bytes = [0u8; 8192];
        let n = self.remaining.min(buf.remaining()).min(bytes.len());
        let mut read = ReadBuf::new(&mut bytes[..n]);
        match Pin::new(&mut self.stream).poll_read(cx, &mut read) {
            Poll::Ready(Ok(())) => {
                let got = read.filled();
                self.remaining -= got.len();
                for byte in got.iter().take(3 - self.greeting.len()) {
                    self.greeting.push(*byte);
                }
                if self.greeting.len() == 3 && self.greeting != b"220" {
                    return Poll::Ready(Err(io::Error::other("invalid SMTP greeting")));
                }
                buf.put_slice(got);
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}
impl AsyncWrite for BoundedStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // Lettre's error cleanup attempts QUIT before returning the original rejection.
        // This one-attempt stream permits no graceful cleanup roundtrip: refuse QUIT
        // locally so a relay cannot replace a known rejection with a cleanup timeout.
        if buf == b"QUIT\r\n" {
            return Poll::Ready(Err(io::Error::other("SMTP cleanup is local")));
        }
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // The enclosing attempt drops the TLS/TCP stream immediately after the error.
        // Do not await a TLS close-notify flush while preserving the original outcome.
        Poll::Ready(Ok(()))
    }
}
