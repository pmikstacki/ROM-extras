/// Finite host observation kinds; these are not inferred physical commit events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// One host action invocation, including replay.
    Action,
    /// One host query invocation.
    Query,
    /// One host Work attempt or processing invocation.
    Work,
    /// One host recovery or reconciliation observation.
    Recovery,
}
impl Operation {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Action => "action",
            Self::Query => "query",
            Self::Work => "work",
            Self::Recovery => "recovery",
        }
    }
}
