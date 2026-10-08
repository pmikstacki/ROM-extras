use std::fmt;

/// Execution-boundary failures. Database commit outcomes remain driver-owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutorError {
    /// Queue capacity must be in 1..=65_536. No connection was initialized.
    InvalidCapacity,
    /// The worker or its connection could not be initialized before admission.
    Initialization,
    /// The queue was full. The submitted operation was not admitted.
    Overloaded,
    /// Admission closed, or an admitted queued operation never reached execution.
    Closed,
    /// The outcome was not established. Do not infer database rollback.
    Unknown,
    /// A connection operation attempted to use its own worker recursively.
    Reentrant,
    /// The ticket's only terminal response was already consumed.
    OutcomeConsumed,
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, formatter)
    }
}

impl std::error::Error for ExecutorError {}
