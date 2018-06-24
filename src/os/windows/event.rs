use std::mem;
use std::ptr;
use timeout::Timeout;
use error::Result;
use event::{Event, UnprocessedEvent};
use super::winapi_utils::*;
use super::{OsContext, OsSharedContext};

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
        unsafe {
            let mut msg = mem::uninitialized();
            match timeout.duration() {
                None => {
                    let ret = GetMessageW(&mut msg, ptr::null_mut(), 0, 0);
                    if ret < 0 {
                        panic!("GetMessageW() failed");
                    }
                    if ret == 0 {
                        self.push_event(Event::Quit);
                    } else {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                },
                Some(timeout) => {
                    let has_one = PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) != FALSE;
                    if has_one {
                        if msg.message == WM_QUIT {
                            self.push_event(Event::Quit);
                        } else {
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                },
            };
        }
        self.pending_events.borrow_mut().pop_front()
    }
}

impl OsSharedContext {
    pub fn push_event(&self, ev: Event) {
        self.pending_events.borrow_mut().push_back(ev);
    }
}
