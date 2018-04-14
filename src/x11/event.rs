use std::mem;
use std::time::Instant;
use super::context::X11SharedContext;
use super::x11::xlib as x;
use super::x11::xinput2 as xi2;
use error::{Result, failed};
use event::Event;
use timeout::Timeout;

impl X11SharedContext {
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        unimplemented!{}
    }
    pub fn next_event(&self, timeout: Timeout) -> Option<Event> {
        unsafe {
            match timeout.duration() {
                None => loop {
                    if let Some(e) = self.wait_next_event() {
                        return Some(e);
                    }
                },
                Some(duration) => {
                    // Welp, just poll repeatedly instead
                    let start = Instant::now();
                    loop {
                        if x::XPending(self.x_display) <= 0 {
                            if Instant::now().duration_since(start) >= duration {
                                return None; // Timed out
                            }
                            continue; // Try again
                        }
                        if let Some(e) = self.wait_next_event() {
                            return Some(e);
                        }
                    }
                },
            }
        }
    }
}


impl X11SharedContext {
    fn wait_next_event(&self) -> Option<Event> {
        let mut x_event = unsafe {
            let mut x_event: x::XEvent = mem::zeroed();
            x::XNextEvent(self.x_display, &mut x_event);
            x_event
        };
        match self.translate_x_event(&mut x_event) {
            Ok(e) => Some(e),
            Err(e) => {
                warn!("Some X event was not handled properly: {}", e);
                None
            },
        }
    }

    fn translate_x_event(&self, e: &mut x::XEvent) -> Result<Event> {
        if let Ok(xi) = self.xi() {
            unsafe {
                let x_display = self.x_display;
                let mut cookie = x::XGenericEventCookie::from(&*e);
                if x::XGetEventData(x_display, &mut cookie) == x::True {
                    if cookie.type_ == x::GenericEvent && cookie.extension == xi.major_opcode {
                        let e = self.translate_xi_event(&mut *(cookie.data as *mut xi2::XIEvent));
                        x::XFreeEventData(x_display, &mut cookie);
                        return e;
                    }
                }
                // NOTE: Yes, do it even if XGetEventData() failed! See the man page.
                x::XFreeEventData(x_display, &mut cookie); 
            }
        }

        match e.get_type() {
            x::ClientMessage => self.translate_x_client_message_event(e.as_mut()),
            x::KeyPress | x::KeyRelease => self.translate_x_key_event(e.as_mut()),
            x::ButtonPress | x::ButtonRelease => self.translate_x_button_event(e.as_mut()),
            x::MotionNotify => self.translate_x_motion_event(e.as_mut()),
            x::EnterNotify | x::LeaveNotify => self.translate_x_crossing_event(e.as_mut()),
            x::FocusIn | x::FocusOut => self.translate_x_focus_change_event(e.as_mut()),
            x::GravityNotify => self.translate_x_gravity_event(e.as_mut()),
            x::ConfigureNotify => self.translate_x_configure_event(e.as_mut()),
            x::MappingNotify => self.translate_x_mapping_event(e.as_mut()),
            x::KeymapNotify 
            | x::NoExpose
            | x::CirculateRequest
            | x::ConfigureRequest
            | x::MapRequest
            | x::ResizeRequest
            | x::CirculateNotify
            | x::CreateNotify
            | x::DestroyNotify
            | x::MapNotify
            | x::ReparentNotify  
            | x::UnmapNotify     
            | x::VisibilityNotify
            | x::ColormapNotify  
            | x::PropertyNotify  
            | x::SelectionClear  
            | x::SelectionNotify 
            | x::SelectionRequest
                => failed(format!("Event that should have been handled, but wasn't: {:?}", e)),
            _   => failed(format!("Unlisted event {:?}", e)),
        }
    }

    fn translate_xi_event(&self, e: &mut xi2::XIEvent) -> Result<Event> {
        match e.evtype {
            xi2::XI_DeviceChanged => self.translate_xi_device_changed_event(unsafe { mem::transmute(e) }),
            xi2::XI_HierarchyChanged => self.translate_xi_hierarchy_event(unsafe { mem::transmute(e) }),
            xi2::XI_PropertyEvent => self.translate_xi_property_event(unsafe { mem::transmute(e) }),
              xi2::XI_Enter
            | xi2::XI_Leave
            | xi2::XI_FocusIn
            | xi2::XI_FocusOut
                => self.translate_xi_enter_event(unsafe { mem::transmute(e) }),
              xi2::XI_KeyPress
            | xi2::XI_KeyRelease
            | xi2::XI_ButtonPress
            | xi2::XI_ButtonRelease
            | xi2::XI_Motion
            | xi2::XI_TouchBegin
            | xi2::XI_TouchUpdate
            | xi2::XI_TouchEnd
                => self.translate_xi_device_event(unsafe { mem::transmute(e) }),
              xi2::XI_RawKeyPress     
            | xi2::XI_RawKeyRelease   
            | xi2::XI_RawButtonPress  
            | xi2::XI_RawButtonRelease
            | xi2::XI_RawMotion       
            | xi2::XI_RawTouchBegin   
            | xi2::XI_RawTouchUpdate  
            | xi2::XI_RawTouchEnd 
                => self.translate_xi_raw_event(unsafe { mem::transmute(e) }),
            _   => failed(format!("Unrecognized XI event: {:?}", e)),
        }
    }

    fn translate_x_motion_event(&self, e: &mut x::XMotionEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_x_crossing_event(&self, e: &mut x::XCrossingEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_x_focus_change_event(&self, e: &mut x::XFocusChangeEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_x_gravity_event(&self, e: &mut x::XGravityEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_x_configure_event(&self, e: &mut x::XConfigureEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_x_mapping_event(&self, e: &mut x::XMappingEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_x_client_message_event(&self, e: &mut x::XClientMessageEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_x_key_event(&self, e: &mut x::XKeyEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_x_button_event(&self, e: &mut x::XButtonEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_xi_device_changed_event(&self, e: &mut xi2::XIDeviceChangedEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_xi_hierarchy_event(&self, e: &mut xi2::XIHierarchyEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_xi_enter_event(&self, e: &mut xi2::XIEnterEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_xi_property_event(&self, e: &mut xi2::XIPropertyEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_xi_device_event(&self, e: &mut xi2::XIDeviceEvent) -> Result<Event> {
        unimplemented!{}
    }
    fn translate_xi_raw_event(&self, e: &mut xi2::XIRawEvent) -> Result<Event> {
        unimplemented!{}
    }
}
