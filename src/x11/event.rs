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
            x::KeyPress | x::KeyRelease => self.translate_x_key_event(e.as_mut()),
            x::ButtonPress | x::ButtonRelease => self.translate_x_button_event(e.as_mut()),
            x::MotionNotify => self.translate_x_motion_event(e.as_mut()),
            x::EnterNotify | x::LeaveNotify => self.translate_x_crossing_event(e.as_mut()),
            x::FocusIn | x::FocusOut => self.translate_x_focus_change_event(e.as_mut()),
            // ^ The above events types have better Xinput 2 replacements.
            x::ClientMessage => self.translate_x_client_message_event(e.as_mut()),
            x::GravityNotify => self.translate_x_gravity_event(e.as_mut()),
            x::ConfigureNotify => self.translate_x_configure_event(e.as_mut()),
            x::MappingNotify => self.translate_x_mapping_event(e.as_mut()),
            x::Expose | x::GraphicsExpose => self.translate_x_expose_event(e.as_mut()),
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
        let &mut x::XMotionEvent {
            type_, serial, send_event, display, window, root, subwindow,
            time, x, y, x_root, y_root, state, is_hint, same_screen,
        } = e;
        /*
        let e = Event::MouseMotion {
            mouse: self.main_mouse(),
            timestamp: Timestamp::from_millis(time as _),
            window: WindowHandle::from_x_window(window),
            position: Vec2::new(x as _, y as _),
            root_position: Vec2::new(x_root as _, y_root as _),
        };
        Ok(e)
        */
        unimplemented!{}
    }
    fn translate_x_crossing_event(&self, e: &mut x::XCrossingEvent) -> Result<Event> {
        let &mut x::XCrossingEvent {
            type_, serial, send_event, display, window, root, subwindow,
            time, x, y, x_root, y_root, mode, detail, same_screen, focus, state,
        } = e;
        unimplemented!{}
    }
    fn translate_x_focus_change_event(&self, e: &mut x::XFocusChangeEvent) -> Result<Event> {
        let &mut x::XFocusChangeEvent {
            type_, serial, send_event, display, window, mode, detail,
        } = e;
        unimplemented!{}
    }
    fn translate_x_expose_event(&self, e: &mut x::XExposeEvent) -> Result<Event> {
        let &mut x::XExposeEvent {
            type_, serial, send_event, display, window,
            x, y, width, height, count,
        } = e;
        unimplemented!{}
    }
    fn translate_x_gravity_event(&self, e: &mut x::XGravityEvent) -> Result<Event> {
        let &mut x::XGravityEvent {
            type_, serial, send_event, display, event, window, x, y,
        } = e;
        unimplemented!{}
    }
    fn translate_x_configure_event(&self, e: &mut x::XConfigureEvent) -> Result<Event> {
        let &mut x::XConfigureEvent {
            type_, serial, send_event, display, event, window, x, y,
            width, height, border_width, above, override_redirect,
        } = e;
        unimplemented!{}
    }
    fn translate_x_mapping_event(&self, e: &mut x::XMappingEvent) -> Result<Event> {
        let &mut x::XMappingEvent {
            type_, serial, send_event, display, event,
            request, first_keycode, count,
        } = e;
        unimplemented!{}
    }
    fn translate_x_client_message_event(&self, e: &mut x::XClientMessageEvent) -> Result<Event> {
        let &mut x::XClientMessageEvent {
            type_, serial, send_event, display, window,
            message_type, format, data,
        } = e;
        unimplemented!{}
    }

    fn translate_x_key_event(&self, e: &mut x::XKeyEvent) -> Result<Event> {
        let &mut x::XKeyEvent {
            type_, serial, send_event, display, window, root, subwindow,
            time, x, y, x_root, y_root, state, keycode, same_screen,
        } = e;
        /*
        let window = WindowHandle::from_x_window(window);
        if window.xic.is_none() {
            return failed("");
        }
        let (keysym, text) = self.x_utf8_lookup_string(xic, e);
        let key = Key {
            code: keycode,
            sym: keysym?,
        };
        let keyboard = self.main_keyboard();
        let timestamp = Timestamp::from_millis(time);

        window.set_net_wm_user_time(time);

        let e = match type_ {
            x::KeyRelease => {
                self.previous_x_key_release_time.set(time);
                self.previous_x_key_release_keycode.set(keycode);
                Event::KeyboardKeyReleased {
                    keyboard, window, timestamp, key,
                }
            },
            x::KeyPress => {
                let is_repeat = {
                    self.previous_x_key_release_time.get() == time
                 && self.previous_x_key_release_keycode.get() == keycode
                };
                let is_text = unsafe {
                    x::False == x::XFilterEvent(e as *mut _ as *mut _, 0)
                };
                let text = if is_text { text.ok() } else { None };
                Event::KeyboardKeyPressed {
                    keyboard, window, timestamp, key, is_repeat, text,
                }
            },
            _ => unreachable!{},
        };
        Ok(e)
        */
        unimplemented!{}
    }

    fn translate_x_button_event(&self, e: &mut x::XButtonEvent) -> Result<Event> {
        let &mut x::XButtonEvent {
            type_, serial, send_event, display, window, root, subwindow,
            time, x, y, x_root, y_root, state, button, same_screen,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_device_changed_event(&self, e: &mut xi2::XIDeviceChangedEvent) -> Result<Event> {
        let &mut xi2::XIDeviceChangedEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, sourceid, reason, num_classes, classes,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_hierarchy_event(&self, e: &mut xi2::XIHierarchyEvent) -> Result<Event> {
        let &mut xi2::XIHierarchyEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, flags, num_info, info,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_enter_event(&self, e: &mut xi2::XIEnterEvent) -> Result<Event> {
        let &mut xi2::XIEnterEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, sourceid, detail, root, event: x_window, child,
            root_x, root_y,
            event_x, event_y,
            mode, focus, same_screen, buttons, mods, group,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_property_event(&self, e: &mut xi2::XIPropertyEvent) -> Result<Event> {
        let &mut xi2::XIPropertyEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, property, what,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_device_event(&self, e: &mut xi2::XIDeviceEvent) -> Result<Event> {
        let &mut xi2::XIDeviceEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, sourceid, detail, root,
            event: x_window, child, root_x, root_y,
            event_x, event_y, flags, buttons, valuators, mods, group,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_raw_event(&self, e: &mut xi2::XIRawEvent) -> Result<Event> {
        let &mut xi2::XIRawEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, sourceid, detail, flags, valuators, raw_values,
        } = e;
        unimplemented!{}
    }

    /*
    fn x_utf8_lookup_string(&self, xic: x::XIC, x_event: &x::XKeyEvent) -> (Result<x::KeySym>, Result<String>) {
        // Asserting because of undefined behaviour otherwise.
        assert_ne!(x_event.type_, x::KeyRelease);
        unsafe {
            let mut buf = Vec::<c_char>::with_capacity(32);
            let mut keysym: x::KeySym = 0;
            let mut status: x::Status = 0;
            loop {
                let actual_len = x::Xutf8LookupString(
                    xic, x_event as *const _ as *mut _,
                    buf.as_mut_ptr(), buf.capacity() as _,
                    &mut keysym, &mut status
                );
                match status {
                    x::XBufferOverflow => {
                        buf.reserve_exact(actual_len as _);
                        continue;
                    },
                    x::XLookupNone => return (None, None),
                    x::XLookupKeySym => return (Some(keysym), None),
                    x::XLookupChars => (),
                    x::XLookupBoth => (),
                    _ => unreachable!{},
                };
                buf.set_len(actual_len as _);
                let s = String::from_utf8(buf).unwrap();
                match status {
                    x::XLookupChars => return (None, Some(s)),
                    x::XLookupBoth => return (Some(keysym), Some(s)),
                    _ => unreachable!{},
                }
            }
        };
    }
    */
}
