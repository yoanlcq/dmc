use super::OsContext;
use timeout::Timeout;
use error::Result;
use event::Event;

#[derive(Debug, Clone, PartialEq)]
pub struct OsUnprocessedEvent;

impl UnprocessedEvent {
    // TODO: Add Emscripten-specific getters here
}

impl OsContext {
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn next_event(&self, timeout: Timeout) -> Option<Event> {
        unimplemented!()
    }
}

