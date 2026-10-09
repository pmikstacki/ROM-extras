//! Exact-value digest encoding, deliberately distinct from JCS and provider wire JSON.
use crate::{Error, Result};
use rom::Value;
use std::fmt::Write as _;
const BYTES: usize = 16 * 1024;
const NODES: usize = 4096;
const DEPTH: usize = 32;
struct Writer {
    bytes: Vec<u8>,
    nodes: usize,
}
impl Writer {
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.len() > BYTES - self.bytes.len() {
            return Err(Error::TooLarge);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn tag(&mut self, tag: u8) -> Result<()> {
        self.write(&[tag])
    }
    fn count(&mut self, count: usize) -> Result<()> {
        self.write(
            &u32::try_from(count)
                .map_err(|_| Error::TooLarge)?
                .to_be_bytes(),
        )
    }
    fn string(&mut self, s: &str) -> Result<()> {
        self.count(s.len())?;
        self.write(s.as_bytes())
    }
    fn value(&mut self, value: &Value, depth: usize) -> Result<()> {
        if depth > DEPTH || self.nodes >= NODES {
            return Err(Error::TooLarge);
        }
        self.nodes += 1;
        match value {
            Value::Null => self.tag(0),
            Value::Bool(false) => self.tag(1),
            Value::Bool(true) => self.tag(2),
            Value::Number(n) => {
                // Bound feature-dependent number text before conversions scan it.
                write!(&mut NumberLength(0), "{n}").map_err(|_| Error::TooLarge)?;
                if let Some(n) = n.as_u64() {
                    if value != &Value::from(n) {
                        return Err(Error::Invalid);
                    }
                    self.tag(3)?;
                    self.write(&n.to_be_bytes())
                } else if let Some(n) = n.as_i64() {
                    if value != &Value::from(n) {
                        return Err(Error::Invalid);
                    }
                    self.tag(4)?;
                    self.write(&n.to_be_bytes())
                } else {
                    if !n.is_f64() {
                        return Err(Error::Invalid);
                    }
                    let n = n.as_f64().filter(|n| n.is_finite()).ok_or(Error::Invalid)?;
                    if Value::from(n).as_number() != value.as_number() {
                        return Err(Error::Invalid);
                    }
                    self.tag(5)?;
                    self.write(&(if n == 0.0 { 0.0 } else { n }).to_bits().to_be_bytes())
                }
            }
            Value::String(s) => {
                self.tag(6)?;
                self.string(s)
            }
            Value::Array(values) => {
                if values.len() > NODES - self.nodes {
                    return Err(Error::TooLarge);
                }
                self.tag(7)?;
                self.count(values.len())?;
                for v in values {
                    self.value(v, depth + 1)?;
                }
                Ok(())
            }
            Value::Object(values) => {
                if values.len() > NODES - self.nodes {
                    return Err(Error::TooLarge);
                }
                let mut entries: Vec<_> = values.iter().collect();
                entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
                self.tag(8)?;
                self.count(entries.len())?;
                for (k, v) in entries {
                    self.string(k)?;
                    self.value(v, depth + 1)?;
                }
                Ok(())
            }
        }
    }
}
struct NumberLength(usize);
impl std::fmt::Write for NumberLength {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.0 = self
            .0
            .checked_add(value.len())
            .filter(|n| *n <= 64)
            .ok_or(std::fmt::Error)?;
        Ok(())
    }
}
pub(crate) fn fields(fields: &[(&str, &Value)]) -> Result<Vec<u8>> {
    let mut writer = Writer {
        bytes: Vec::new(),
        nodes: 1,
    };
    writer.tag(8)?;
    writer.count(fields.len())?;
    for (k, v) in fields {
        writer.string(k)?;
        writer.value(v, 1)?;
    }
    Ok(writer.bytes)
}
