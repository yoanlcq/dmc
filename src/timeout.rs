//! The Timeout enum, which is either a fixed duration or infinite.

use std::time::Duration;

///! The Timeout enum, which is either a fixed duration or infinite.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Timeout {
    Set(Duration),
    Infinite,
}
impl From<Duration> for Timeout {
    fn from(d: Duration) -> Self {
        Timeout::Set(d)
    }
}

