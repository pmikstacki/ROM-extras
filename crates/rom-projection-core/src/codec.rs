//! Versioned bounded fixed-field records; no general-purpose deserializer.
use crate::metadata::*;
use crate::{Error, Result, limits};
use rom::{JournalCursor, Key};
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 7] = b"ROMXPRJ";

struct Writer(Vec<u8>);
impl Writer {
    fn new(tag: u8) -> Self {
        let mut writer = Self(MAGIC.to_vec());
        writer.0.extend_from_slice(&1u16.to_be_bytes());
        writer.byte(tag);
        writer
    }
    fn byte(&mut self, value: u8) {
        self.0.push(value);
    }
    fn number(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }
    fn count(&mut self, value: usize) {
        self.0.extend_from_slice(&(value as u16).to_be_bytes());
    }
    fn string(&mut self, value: &str) {
        self.count(value.len());
        self.0.extend_from_slice(value.as_bytes());
    }
    fn cursor(&mut self, value: &JournalCursor) {
        self.string(&value.kind);
        self.string(&value.generation);
        self.number(value.position);
    }
    fn checkpoint(&mut self, value: &Checkpoint) {
        self.string(&value.physical);
        self.number(value.sequence);
        self.count(value.cursors.len());
        for cursor in &value.cursors {
            self.cursor(cursor);
        }
    }
    fn operation(&mut self, value: &OperationMetadata) {
        self.string(&value.key.kind);
        self.string(&value.key.id);
        self.number(value.position);
        self.number(value.revision);
        self.byte(u8::from(value.tombstone));
        self.0.extend_from_slice(&value.digest);
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], tag: u8, limit: usize) -> Result<Self> {
        if bytes.len() > limit {
            return Err(Error::TooLarge);
        }
        let mut reader = Self { bytes, position: 0 };
        if reader.take(7)? != MAGIC
            || reader.take(2)? != 1u16.to_be_bytes()
            || reader.byte()? != tag
        {
            return Err(Error::Unsupported);
        }
        Ok(reader)
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self.position.checked_add(count).ok_or(Error::Corrupt)?;
        let value = self.bytes.get(self.position..end).ok_or(Error::Corrupt)?;
        self.position = end;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn flag(&mut self) -> Result<bool> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::Corrupt),
        }
    }
    fn number(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().map_err(|_| Error::Corrupt)?,
        ))
    }
    fn count(&mut self, limit: usize) -> Result<usize> {
        let value = usize::from(u16::from_be_bytes(
            self.take(2)?.try_into().map_err(|_| Error::Corrupt)?,
        ));
        if value > limit {
            return Err(Error::TooLarge);
        }
        Ok(value)
    }
    fn string(&mut self, limit: usize) -> Result<String> {
        let count = self.count(limit)?;
        let value = std::str::from_utf8(self.take(count)?).map_err(|_| Error::Corrupt)?;
        limits::identifier(value, limit).map_err(|_| Error::Corrupt)?;
        Ok(value.into())
    }
    fn digest(&mut self) -> Result<[u8; 32]> {
        self.take(32)?.try_into().map_err(|_| Error::Corrupt)
    }
    fn cursor(&mut self) -> Result<JournalCursor> {
        Ok(JournalCursor {
            kind: self.string(limits::IDENTIFIER)?,
            generation: self.string(limits::IDENTIFIER)?,
            position: self.number()?,
        })
    }
    fn checkpoint(&mut self) -> Result<Checkpoint> {
        let physical = self.string(limits::IDENTIFIER)?;
        let sequence = self.number()?;
        let count = self.count(limits::KINDS)?;
        let mut cursors = Vec::with_capacity(count);
        for _ in 0..count {
            cursors.push(self.cursor()?);
        }
        if cursors.windows(2).any(|pair| pair[0].kind >= pair[1].kind) {
            return Err(Error::Corrupt);
        }
        let mut checkpoint = Checkpoint::new(&physical, cursors).map_err(|_| Error::Corrupt)?;
        checkpoint.sequence = sequence;
        Ok(checkpoint)
    }
    fn operation(&mut self) -> Result<OperationMetadata> {
        let key = Key {
            kind: self.string(limits::IDENTIFIER)?,
            id: self.string(limits::RESOURCE_ID)?,
        };
        let position = self.number()?;
        let revision = self.number()?;
        let tombstone = self.flag()?;
        let digest = self.digest()?;
        OperationMetadata::new(key, position, revision, tombstone, digest)
            .map_err(|_| Error::Corrupt)
    }
    fn finish(self) -> Result<()> {
        if self.position != self.bytes.len() {
            return Err(Error::Corrupt);
        }
        Ok(())
    }
}

pub(crate) fn profile(value: &ProjectionProfile) -> Vec<u8> {
    let mut writer = Writer::new(1);
    writer.string(&value.deployment);
    writer.string(&value.provider);
    writer.string(&value.mapping);
    writer.byte(u8::from(value.model.is_some()));
    if let Some(model) = &value.model {
        writer.string(model);
    }
    for bound in [
        limits::OPERATIONS as u64,
        limits::KINDS as u64,
        limits::IDENTIFIER as u64,
        limits::RESOURCE_ID as u64,
        limits::RECORD as u64,
        limits::KEY_RECORD as u64,
        limits::SCAN_SEGMENT as u64,
        limits::CACHE as u64,
        limits::RECOVERY_FILE,
    ] {
        writer.number(bound);
    }
    writer.0
}
pub(crate) fn validate_profile(bytes: &[u8], expected: &ProjectionProfile) -> Result<()> {
    if bytes.len() > limits::RECORD {
        return Err(Error::TooLarge);
    }
    if bytes != profile(expected) {
        return Err(Error::Unsupported);
    }
    Ok(())
}
pub(crate) fn state(value: &StoreSnapshot) -> Vec<u8> {
    let mut writer = Writer::new(2);
    writer.checkpoint(&value.checkpoint);
    writer.byte(u8::from(value.completed.is_some()));
    if let Some((prepare, complete)) = &value.completed {
        writer.0.extend_from_slice(&token(prepare));
        writer.0.extend_from_slice(&token(complete));
    }
    writer.0
}
pub(crate) fn decode_state(bytes: &[u8]) -> Result<StoreSnapshot> {
    let mut reader = Reader::new(bytes, 2, limits::RECORD)?;
    let checkpoint = reader.checkpoint()?;
    let completed = if reader.flag()? {
        Some((
            decode_token(reader.take(83)?)?,
            decode_token(reader.take(83)?)?,
        ))
    } else {
        None
    };
    reader.finish()?;
    Ok(StoreSnapshot {
        checkpoint,
        pending: None,
        completed,
        completed_page: None,
    })
}
pub(crate) fn page(value: &PageIntent) -> Vec<u8> {
    let mut writer = Writer::new(3);
    writer.checkpoint(&value.expected);
    writer.cursor(&value.next);
    writer.count(value.operations.len());
    for operation in &value.operations {
        writer.operation(operation);
    }
    writer.0
}
pub(crate) fn decode_page(bytes: &[u8]) -> Result<PageIntent> {
    let mut reader = Reader::new(bytes, 3, limits::RECORD)?;
    let expected = reader.checkpoint()?;
    let next = reader.cursor()?;
    let count = reader.count(limits::OPERATIONS)?;
    let mut operations = Vec::with_capacity(count);
    for _ in 0..count {
        operations.push(reader.operation()?);
    }
    reader.finish()?;
    // Constructor validates arrival order. Stored operations use canonical key order.
    let canonical = operations.clone();
    operations.sort_by_key(|operation| operation.position);
    let value = PageIntent::new(&expected, next, operations).map_err(|_| Error::Corrupt)?;
    if value.operations != canonical || page(&value) != bytes {
        return Err(Error::Corrupt);
    }
    Ok(value)
}
pub(crate) fn key(value: &Key) -> Vec<u8> {
    let mut writer = Writer(Vec::new());
    writer.string(&value.kind);
    writer.string(&value.id);
    writer.0
}
pub(crate) fn decode_key(bytes: &[u8]) -> Result<Key> {
    if bytes.len() > limits::IDENTIFIER + limits::RESOURCE_ID + 4 {
        return Err(Error::TooLarge);
    }
    let mut reader = Reader { bytes, position: 0 };
    let key = Key {
        kind: reader.string(limits::IDENTIFIER)?,
        id: reader.string(limits::RESOURCE_ID)?,
    };
    reader.finish()?;
    Ok(key)
}
pub(crate) fn key_state(value: &KeyState) -> Vec<u8> {
    let mut writer = Writer::new(5);
    writer.number(value.revision);
    writer.byte(u8::from(value.tombstone));
    writer.0.extend_from_slice(&value.digest);
    writer.0
}
pub(crate) fn decode_key_state(bytes: &[u8]) -> Result<KeyState> {
    let mut reader = Reader::new(bytes, 5, limits::KEY_RECORD)?;
    let revision = reader.number()?;
    let tombstone = reader.flag()?;
    let digest = reader.digest()?;
    reader.finish()?;
    if revision == 0 {
        return Err(Error::Corrupt);
    }
    Ok(KeyState {
        revision,
        tombstone,
        digest,
    })
}
pub(crate) fn token(value: &TransactionId) -> [u8; 83] {
    let mut writer = Writer::new(6);
    writer.number(value.sequence);
    writer.byte(value.transition);
    writer.0.extend_from_slice(&value.fingerprint);
    writer.0.extend_from_slice(&value.previous);
    writer.0.try_into().expect("fixed token encoding")
}
pub(crate) fn decode_token(bytes: &[u8]) -> Result<TransactionId> {
    if bytes.len() != 83 {
        return Err(Error::Invalid);
    }
    let mut reader = Reader::new(bytes, 6, 83)?;
    let sequence = reader.number()?;
    let transition = reader.byte()?;
    let fingerprint = reader.digest()?;
    let previous = reader.digest()?;
    reader.finish()?;
    if sequence == 0 || !matches!(transition, 1 | 2) {
        return Err(Error::Invalid);
    }
    Ok(TransactionId {
        sequence,
        transition,
        fingerprint,
        previous,
    })
}
fn digest(domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(domain);
    for field in fields {
        hash.update((field.len() as u64).to_be_bytes());
        hash.update(field);
    }
    hash.finalize().into()
}
pub(crate) fn page_digest(profile_value: &ProjectionProfile, page_value: &PageIntent) -> [u8; 32] {
    digest(
        b"ROM-extras/projection-intent/v1",
        &[&profile(profile_value), &page(page_value)],
    )
}
pub(crate) fn control_digest(profile_value: &ProjectionProfile, value: &StoreSnapshot) -> [u8; 32] {
    let pending = value.pending.as_ref().map(page).unwrap_or_default();
    let completed_page = value.completed_page.as_ref().map(page).unwrap_or_default();
    digest(
        b"ROM-extras/projection-control-state/v1",
        &[
            &profile(profile_value),
            &state(value),
            &pending,
            &completed_page,
        ],
    )
}
