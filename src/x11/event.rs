use super::context::X11SharedContext;
use event::Event;
use timeout::Timeout;

impl X11SharedContext {
    pub fn next_event(&self, timeout: Timeout) -> Option<Event> {
        unimplemented!{}
    }
}
