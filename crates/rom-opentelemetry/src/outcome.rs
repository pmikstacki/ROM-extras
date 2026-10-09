/// Finite observed outcomes. Generic failure does not establish rollback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The observed host call succeeded, not a distinct physical commit count.
    Success,
    /// Current authority rejected the operation.
    Denied,
    /// An expected revision or equivalent condition conflicted.
    Conflict,
    /// Admission capacity was unavailable.
    Overloaded,
    /// ROM explicitly established that the operation was not committed.
    NotCommitted,
    /// An effect or commit remains uncertain.
    Unknown,
    /// A typed validation or unsupported-input class rejected the call.
    Rejected,
    /// A host-visible failure without a rollback assertion.
    Failed,
    /// The runtime intake was closed or otherwise explicitly unavailable.
    Unavailable,
    /// The external provider acknowledged acceptance, not end-user delivery.
    Accepted,
    /// The delivery adapter reported an explicitly retryable outcome.
    Retryable,
    /// The delivery adapter reported permanent rejection.
    Permanent,
    /// Observation ended without a result; no rollback or cancellation is inferred.
    Abandoned,
    /// Runtime delivery deadline expired; external effects can remain uncertain.
    TimedOut,
    /// An observation panicked; this does not establish rollback or no external effect.
    Panicked,
}
impl Outcome {
    /// Classify only the typed result variant; never inspect, retain or format values.
    pub fn from_result<T>(result: &rom::Result<T>) -> Self {
        match result {
            Ok(_) => Self::Success,
            Err(rom::Error::Denied) => Self::Denied,
            Err(rom::Error::Conflict) => Self::Conflict,
            Err(rom::Error::Overloaded) => Self::Overloaded,
            Err(rom::Error::NotCommitted) => Self::NotCommitted,
            Err(rom::Error::Unknown) => Self::Unknown,
            Err(rom::Error::Closed) => Self::Unavailable,
            Err(rom::Error::Storage) => Self::Failed,
            Err(rom::Error::Panicked) => Self::Panicked,
            Err(_) => Self::Rejected,
        }
    }
    /// Preserve the public provider's delivery classification without interpreting receipt data.
    pub fn from_delivery(outcome: rom::DeliveryOutcome) -> Self {
        match outcome {
            rom::DeliveryOutcome::Accepted => Self::Accepted,
            rom::DeliveryOutcome::Retryable => Self::Retryable,
            rom::DeliveryOutcome::Permanent => Self::Permanent,
            rom::DeliveryOutcome::Unknown => Self::Unknown,
            rom::DeliveryOutcome::TimedOut => Self::TimedOut,
            rom::DeliveryOutcome::Panicked => Self::Panicked,
        }
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Denied => "denied",
            Self::Conflict => "conflict",
            Self::Overloaded => "overloaded",
            Self::NotCommitted => "not_committed",
            Self::Unknown => "unknown",
            Self::Rejected => "rejected",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
            Self::Accepted => "accepted",
            Self::Retryable => "retryable",
            Self::Permanent => "permanent",
            Self::Abandoned => "abandoned",
            Self::TimedOut => "timed_out",
            Self::Panicked => "panicked",
        }
    }
}
