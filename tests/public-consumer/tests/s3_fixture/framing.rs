//! Strict bounded framing for the local HTTP/1 fault fixture.
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};
const HEADER_MAX: usize = 32 * 1024;
const BODY_MAX: usize = 4096;
const IO_BUDGET: Duration = Duration::from_secs(2);

pub(super) fn header(stream: &mut TcpStream) -> Result<Vec<u8>, &'static str> {
    let deadline = Instant::now() + IO_BUDGET;
    let mut bytes = Vec::new();
    while bytes.len() < HEADER_MAX {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("header deadline")?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| "read timeout setup")?;
        let mut byte = [0];
        stream
            .read_exact(&mut byte)
            .map_err(|_| "incomplete HTTP header")?;
        bytes.push(byte[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            return Ok(bytes);
        }
    }
    Err("header limit")
}
pub(super) fn text(bytes: &[u8]) -> Result<&str, &'static str> {
    std::str::from_utf8(bytes).map_err(|_| "non-ASCII fixture header")
}
pub(super) fn length(header: &str, required: bool) -> Result<usize, &'static str> {
    let mut length = None;
    for line in header.split("\r\n").skip(1).filter(|l| !l.is_empty()) {
        let (name, value) = line.split_once(':').ok_or("invalid header field")?;
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err("unsupported transfer encoding");
        }
        if name.eq_ignore_ascii_case("expect") {
            return Err("unsupported expectation");
        }
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err("duplicate content length");
            }
            let size = value
                .trim()
                .parse::<usize>()
                .map_err(|_| "invalid content length")?;
            if size > BODY_MAX {
                return Err("body limit");
            }
            length = Some(size);
        }
    }
    if required && length.is_none() {
        return Err("missing content length");
    }
    Ok(length.unwrap_or(0))
}
pub(super) fn body(stream: &mut TcpStream, size: usize) -> Result<Vec<u8>, &'static str> {
    stream
        .set_read_timeout(Some(IO_BUDGET))
        .map_err(|_| "body timeout setup")?;
    // A finite number of read_exact calls would still allow slow progress; read under one deadline.
    let deadline = Instant::now() + IO_BUDGET;
    let mut bytes = vec![0; size];
    let mut offset = 0;
    while offset < size {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("body deadline")?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| "body timeout setup")?;
        let count = stream
            .read(&mut bytes[offset..])
            .map_err(|_| "incomplete body")?;
        if count == 0 {
            return Err("incomplete body");
        }
        offset += count;
    }
    Ok(bytes)
}
pub(super) fn write(stream: &mut TcpStream, bytes: &[u8]) -> Result<(), &'static str> {
    let deadline = Instant::now() + IO_BUDGET;
    let mut offset = 0;
    while offset < bytes.len() {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("write deadline")?;
        stream
            .set_write_timeout(Some(remaining))
            .map_err(|_| "write timeout setup")?;
        let count = stream
            .write(&bytes[offset..])
            .map_err(|_| "write failure")?;
        if count == 0 {
            return Err("write closed");
        }
        offset += count;
    }
    Ok(())
}

// This fixture closes after each response; advertise that hop's connection semantics.
pub(super) fn closing_response(header: &[u8]) -> Result<Vec<u8>, &'static str> {
    let text = text(header)?;
    let mut nominated = vec!["connection", "keep-alive"];
    for line in text.split("\r\n").skip(1) {
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("connection")
        {
            nominated.extend(value.split(',').map(str::trim));
        }
    }
    if nominated.iter().any(|name| {
        name.eq_ignore_ascii_case("content-length")
            || name.eq_ignore_ascii_case("transfer-encoding")
    }) {
        return Err("connection nominates framing field");
    }
    let mut result = String::new();
    for line in text.split("\r\n").filter(|line| !line.is_empty()) {
        if let Some((name, _)) = line.split_once(':')
            && nominated
                .iter()
                .any(|candidate| name.eq_ignore_ascii_case(candidate))
        {
            continue;
        }
        result.push_str(line);
        result.push_str("\r\n");
    }
    result.push_str("Connection: close\r\n\r\n");
    Ok(result.into_bytes())
}
