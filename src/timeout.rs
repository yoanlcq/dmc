//! Convenience type for representing a timeout, either infinite or finite.

use std::time::Duration;

/// Convenience type for representing a timeout, either infinite or finite.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Timeout {
    pub(crate) duration: Option<Duration>,
}

impl Timeout {
    /// Creates an infinite timeout.
    pub fn infinite() -> Self { Self { duration: None } }
    /// Creates an empty (zero) timeout, in fact, no timeout so to speak.
    pub fn none() -> Self { Duration::default().into() }
    /// Is this timeout infinite?
    pub fn is_infinite(&self) -> bool { self.duration.is_none() }
    /// This timeout's duration, if not infinite.
    pub fn duration(&self) -> Option<Duration> { self.duration }
}

impl From<Duration> for Timeout {
    fn from(duration: Duration) -> Self { Self { duration: Some(duration) } }
}
