use crate::Error;
/// Immutable admission limits. Byte admission precedes parsing; structural admission follows it.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    bytes: usize,
    depth: usize,
    nodes: usize,
    string_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            bytes: 65_536,
            depth: 32,
            nodes: 4_096,
            string_bytes: 16_384,
        }
    }
}
impl Limits {
    /// Restrict byte, value-depth, value-node and string/key byte counts.
    /// The root has depth one. Object keys do not add value nodes.
    /// Limits cannot exceed the corresponding default hard ceilings.
    pub fn new(
        bytes: usize,
        depth: usize,
        nodes: usize,
        string_bytes: usize,
    ) -> Result<Self, Error> {
        if bytes == 0
            || bytes > 65_536
            || depth == 0
            || depth > 32
            || nodes == 0
            || nodes > 4_096
            || string_bytes == 0
            || string_bytes > 16_384
        {
            return Err(Error::InvalidLimits);
        }
        Ok(Self {
            bytes,
            depth,
            nodes,
            string_bytes,
        })
    }
    pub(crate) fn max_bytes(self) -> usize {
        self.bytes
    }
    pub(crate) fn check_bytes(self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > self.bytes {
            Err(Error::TooLarge)
        } else {
            Ok(())
        }
    }
    pub(crate) fn parse(self, bytes: &[u8]) -> Result<rom::Value, Error> {
        let value = rom::parse_json(bytes).map_err(|_| Error::InvalidJson)?;
        let mut nodes = 0;
        self.visit(&value, 1, &mut nodes)?;
        Ok(value)
    }
    fn visit(self, value: &rom::Value, depth: usize, nodes: &mut usize) -> Result<(), Error> {
        *nodes += 1;
        if depth > self.depth || *nodes > self.nodes {
            return Err(Error::TooLarge);
        }
        match value {
            rom::Value::String(s) if s.len() > self.string_bytes => return Err(Error::TooLarge),
            rom::Value::Array(values) => {
                for v in values {
                    self.visit(v, depth + 1, nodes)?;
                }
            }
            rom::Value::Object(values) => {
                for (key, v) in values {
                    if key.len() > self.string_bytes {
                        return Err(Error::TooLarge);
                    }
                    self.visit(v, depth + 1, nodes)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
