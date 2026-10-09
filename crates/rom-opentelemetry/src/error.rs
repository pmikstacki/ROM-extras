/// Safe recording errors without source values or configuration text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryError {
    /// A native counter cannot be represented by an unsigned 64-bit instrument.
    CountOverflow,
}
impl std::fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("telemetry count is not representable")
    }
}
impl std::error::Error for TelemetryError {}
