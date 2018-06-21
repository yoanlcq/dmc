use super::OsContext;
use timeout::Timeout;
use error::Result;
use event::{Event, UnprocessedEvent};
use super::winapi_utils::*;

#[derive(Debug, Clone, PartialEq)]
pub struct OsUnprocessedEvent {
    umsg: UINT,
    lparam: LPARAM,
    wparam: WPARAM,
}

impl OsUnprocessedEvent {
    fn umsg_lparam_wparam(&self) -> (UINT, LPARAM, WPARAM) {
        (self.umsg, self.lparam, self.wparam)
    }
}
impl UnprocessedEvent {
    /// (Windows-only) Gets the uMsg, LPARAM and WPARAM associated with the event.
    pub fn umsg_lparam_wparam(&self) -> (UINT, LPARAM, WPARAM) {
        self.os_event.umsg_lparam_wparam()
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

