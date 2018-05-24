use super::OsContext;
use timeout::Timeout;
use error::Result;
use event::{Event, SystemEvent};
use super::winapi_utils::*;

#[derive(Debug, Clone, PartialEq)]
pub struct OsSystemEvent {
    umsg: UINT,
    lparam: LPARAM,
    wparam: WPARAM,
}

impl SystemEvent {
    /// (Windows-only) Gets the uMsg, LPARAM and WPARAM associated with the event.
    pub fn umsg_lparam_wparam(&self) -> (UINT, LPARAM, WPARAM) {
        (self.0.umsg, self.0.lparam, self.0.wparam)
    }
}

impl OsContext {
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn next_event(&self, timeout: Timeout) -> Option<Event> {
        unimplemented!()
    }
}

