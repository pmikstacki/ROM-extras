//! Bounded reconstruction of the immutable pending authorized interval.
use crate::{Cancellation, Error, OperationMetadata, PageIntent, Result, codec, limits};
use rom::{JournalBatch, JournalCursor, JournalView};
use std::collections::BTreeMap;
const FETCHES: usize = 64;
const INSPECTED: u64 = 4096;
const VIEWS: usize = 4096;
/// Metadata-only history reconstruction, without checkpoint or provider I/O.
/// The host supplies public authorized journal batches and an immutable deterministic mapping.
/// No projected values are retained. This does not grant export authority or qualify a provider.
pub struct PendingHistory {
    pending: PageIntent,
    cursor: JournalCursor,
    operations: BTreeMap<Vec<u8>, OperationMetadata>,
    fetches: usize,
    views: usize,
    failed: Option<Error>,
}
impl PendingHistory {
    /// Start at the pending intent's original expected cursor, with fixed finite work limits.
    pub fn new(pending: PageIntent) -> Result<Self> {
        let cursor = pending
            .expected
            .cursor(&pending.next.kind)
            .ok_or(Error::HistoryGap)?
            .clone();
        if pending
            .next
            .position
            .checked_sub(cursor.position)
            .ok_or(Error::HistoryGap)?
            > INSPECTED
        {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            pending,
            cursor,
            operations: BTreeMap::new(),
            fetches: 0,
            views: 0,
            failed: None,
        })
    }
    /// Next fetch position, clamped to the immutable endpoint after complete batch validation.
    pub fn cursor(&self) -> &JournalCursor {
        &self.cursor
    }
    pub(crate) fn check_fetch_budget(&self) -> Result<()> {
        if self.fetches >= FETCHES {
            Err(Error::TooLarge)
        } else {
            Ok(())
        }
    }
    /// Validate a complete batch before mapping its old interval.
    /// Limits are 64 fetches, 4096 views and 4096 global inspected positions, including overrun.
    /// A pre-cancelled call leaves state intact. Any other error makes reconstruction terminal.
    /// Mapping receives authorized historical views, including tombstones; it must be deterministic.
    pub fn push(
        &mut self,
        batch: &JournalBatch,
        cancel: &Cancellation,
        mut map: impl FnMut(&JournalView) -> Result<OperationMetadata>,
    ) -> Result<()> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        cancel.check()?;
        // An unwinding host mapper must not leave a reusable partial attempt.
        self.failed = Some(Error::RebuildRequired);
        let result = self.inspect(batch, cancel, &mut map);
        self.failed = result.as_ref().err().cloned();
        result
    }
    fn inspect(
        &mut self,
        batch: &JournalBatch,
        cancel: &Cancellation,
        map: &mut impl FnMut(&JournalView) -> Result<OperationMetadata>,
    ) -> Result<()> {
        if self.cursor.position == self.pending.next.position {
            return Err(Error::Invalid);
        }
        limits::cursor(&batch.cursor)?;
        if batch.cursor.kind != self.cursor.kind
            || batch.cursor.generation != self.cursor.generation
            || batch.cursor.position <= self.cursor.position
        {
            return Err(Error::HistoryGap);
        }
        let start = self
            .pending
            .expected
            .cursor(&self.cursor.kind)
            .ok_or(Error::HistoryGap)?;
        self.check_fetch_budget()?;
        if batch
            .cursor
            .position
            .checked_sub(start.position)
            .ok_or(Error::HistoryGap)?
            > INSPECTED
            || batch.events.len() > VIEWS - self.views
        {
            return Err(Error::TooLarge);
        }
        let mut previous = self.cursor.position;
        // Validate the overrun too. Malformed later rows cannot hide behind interval clamping.
        for event in &batch.events {
            cancel.check()?;
            limits::key(&event.view.key)?;
            if event.view.key.kind != self.cursor.kind
                || event.view.revision == 0
                || event.position <= previous
                || event.position > batch.cursor.position
            {
                return Err(Error::HistoryGap);
            }
            previous = event.position;
        }
        let mut operations = self.operations.clone();
        for event in batch
            .events
            .iter()
            .take_while(|e| e.position <= self.pending.next.position)
        {
            cancel.check()?;
            let operation = map(event)?;
            if operation.key != event.view.key
                || operation.position != event.position
                || operation.revision != event.view.revision
                || operation.tombstone != event.view.value.is_none()
            {
                return Err(Error::RebuildRequired);
            }
            let key = codec::key(&operation.key);
            if let Some(old) = operations.get(&key)
                && (operation.revision < old.revision
                    || (operation.revision == old.revision
                        && operation.key_state() != old.key_state()))
            {
                return Err(Error::RebuildRequired);
            }
            operations.insert(key, operation);
            if operations.len() > limits::OPERATIONS {
                return Err(Error::TooLarge);
            }
        }
        cancel.check()?;
        self.operations = operations;
        self.fetches += 1;
        self.views += batch.events.len();
        self.cursor.position = batch.cursor.position.min(self.pending.next.position);
        Ok(())
    }
    /// Require complete inspected coverage and full reconstructed intent equality.
    /// The returned intent is not a provider acknowledgement or permission to advance a checkpoint.
    pub fn finish(self) -> Result<PageIntent> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if self.cursor != self.pending.next {
            return Err(Error::HistoryGap);
        }
        let mut operations: Vec<_> = self.operations.into_values().collect();
        operations.sort_by_key(|operation| operation.position);
        let reconstructed = PageIntent::new(
            &self.pending.expected,
            self.pending.next.clone(),
            operations,
        )?;
        if reconstructed != self.pending {
            return Err(Error::RebuildRequired);
        }
        Ok(reconstructed)
    }
}
impl std::fmt::Debug for PendingHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PendingHistory")
    }
}
