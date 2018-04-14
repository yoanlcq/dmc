use super::context::X11SharedContext;
use error::Result;
use event::Event;
use timeout::Timeout;

impl X11SharedContext {
    pub fn next_event(&self, timeout: Timeout) -> Option<Event> {
        unimplemented!{}
    }
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        unimplemented!{}
    }
}
