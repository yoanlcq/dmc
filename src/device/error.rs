//! Errors returned by operations from this module and submodules.

use std::fmt::{self, Display, Formatter};
use error::{self, CowStr};
use event::EventInstant;

/// Error returned by operations from this module and submodules.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Error {
    /// The device was disconnected at the specific instant, if known.
    ///
    /// The instant may also be `None` if you already received a `DeviceDiconnected` event
    /// and the implementation decided to discard all data related to the offending `DeviceID`.
    DeviceDisconnected(Option<EventInstant>),
    /// The device (or backend for the device) does not support this operation.
    NotSupportedByDevice { #[allow(missing_docs)] reason: Option<CowStr> },
    /// Another error occured (in the meantime, it is unknown whether or not the device is still connected).
    Other(error::Error),
}

/// Convenience alias to `Result<T, Error>`.
pub type Result<T> = ::std::result::Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Error::DeviceDisconnected(ref instant) => match *instant {
                None => write!(f, "Device disconnected"),
                Some(instant) => write!(f, "Device disconnected at instant {:?}", instant),
            },
            Error::NotSupportedByDevice { ref reason } => match reason.as_ref() {
                None => write!(f, "Not supported by device"),
                Some(reason) => write!(f, "Not supported by device: {}", reason),
            },
            Error::Other(ref e) => write!(f, "{}", e),
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::DeviceDisconnected(_) => "Device disconnected",
            Error::NotSupportedByDevice { reason: _ } => "Not supported by device",
            Error::Other(ref e) => e.description(),
        }
    }
}

#[allow(dead_code)]
pub(crate) fn disconnected_at<T>(instant: EventInstant) -> Result<T> {
    Err(Error::DeviceDisconnected(Some(instant)))
}
#[allow(dead_code)]
pub(crate) fn disconnected<T>() -> Result<T> {
    Err(Error::DeviceDisconnected(None))
}

#[allow(dead_code)]
pub(crate) fn failed_unexplained<T>() -> Result<T> {
    Err(Error::Other(error::Error::failed_unexplained()))
}
#[allow(dead_code)]
pub(crate) fn failed<T, S: Into<CowStr>>(s: S) -> Result<T> {
    Err(Error::Other(error::Error::failed(s)))
}

#[allow(dead_code)]
pub(crate) fn not_supported_by_device<T, S: Into<CowStr>>(s: S) -> Result<T> {
    Err(Error::NotSupportedByDevice { reason: Some(s.into()) })
}
#[allow(dead_code)]
pub(crate) fn not_supported_by_device_unexplained<T>() -> Result<T> {
    Err(Error::NotSupportedByDevice { reason: None })
}

