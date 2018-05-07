use std::mem;
use std::rc::Rc;
use std::os::raw::c_int;
use super::context::X11SharedContext;
use super::x11::xlib as x;
use super::x11::xinput2 as xi2;
use super::{X11SharedWindow, X11DeviceID};
use error::{self, Result, failed};
use event::{Event, Timestamp};
use hid::{HidID, MouseButton, Key, Keysym, Keycode};
use window::WindowHandle;
use {Vec2, Extent2, Rect};

impl X11SharedContext {
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        unimplemented!{}
    }
    pub fn poll_next_event(&self) -> Option<Event> {
        if let Some(e) = self.pending_translated_events.borrow_mut().pop_front() {
            return Some(e);
        }
        while unsafe { x::XPending(self.x_display) } > 0 {
            if let Some(e) = self.wait_next_event() {
                return Some(e);
            }
        }
        None
    }
}

type TranslateEventResult = ::std::result::Result<Event, Option<error::Error>>;

fn discard_event() -> TranslateEventResult {
    Err(None)
}
fn bad_event<S: Into<error::CowStr>>(s: S) -> TranslateEventResult {
    Err(Some(error::Error::failed(s)))
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
                match e {
                    Some(e) => {
                        warn!("Some X event was not handled properly: {}", e)
                    },
                    None => {
                        trace!("Some X event was purposefully discarded: {:?}", x_event)
                    },
                }
                None
            },
        }
    }

    fn translate_x_event(&self, e: &mut x::XEvent) -> TranslateEventResult {
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
        } else {
            // These events are the older couterparts to XI2 events; they don't give as much
            // information.
            // If we have XI2, handle XI2 events instead (see the above 'if').
            // If we don't, still handle them here.
            let e = match e.get_type() {
                x::KeyPress | x::KeyRelease => Some(self.translate_x_key_event(e.as_mut())),
                x::ButtonPress | x::ButtonRelease => Some(self.translate_x_button_event(e.as_mut())),
                x::MotionNotify => Some(self.translate_x_motion_event(e.as_mut())),
                x::EnterNotify | x::LeaveNotify => Some(self.translate_x_crossing_event(e.as_mut())),
                x::FocusIn | x::FocusOut => Some(self.translate_x_focus_change_event(e.as_mut())),
                _ => None,
            };
            if let Some(e) = e {
                return e;
            }
        }

        match e.get_type() {
            x::ClientMessage => self.translate_x_client_message_event(e.as_mut()),
            x::GravityNotify => self.translate_x_gravity_event(e.as_mut()),
            x::ConfigureNotify => self.translate_x_configure_event(e.as_mut()),
            x::MappingNotify => self.translate_x_mapping_event(e.as_mut()),
            x::Expose  => self.translate_x_expose_event(e.as_mut()),
            x::VisibilityNotify => self.translate_x_visibility_event(e.as_mut()),
            x::MapNotify => self.translate_x_map_event(e.as_mut()),
            x::UnmapNotify => self.translate_x_unmap_event(e.as_mut()),
            x::GraphicsExpose
            | x::NoExpose
                => discard_event(),
            x::KeymapNotify 
            | x::CirculateRequest
            | x::ConfigureRequest
            | x::MapRequest
            | x::ResizeRequest
            | x::CirculateNotify
            | x::CreateNotify
            | x::DestroyNotify
            | x::ReparentNotify  
            | x::ColormapNotify  
            | x::PropertyNotify  
            | x::SelectionClear  
            | x::SelectionNotify 
            | x::SelectionRequest
                => bad_event(format!("Unhandled event {:?}", e)),
            _   => bad_event(format!("Unknown event {:?}", e)),
        }
    }

    fn translate_xi_event(&self, e: &mut xi2::XIEvent) -> TranslateEventResult {
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
            _   => bad_event(format!("Unknown XI event: {:?}", e)),
        }
    }

    fn translate_x_map_event(&self, e: &mut x::XMapEvent) -> TranslateEventResult {
        let &mut x::XMapEvent {
            type_: _, serial: _, send_event: _, display: _, event: _, window,
            override_redirect: _,
        } = e;
        Ok(Event::WindowShown { window: WindowHandle(window) })
    }
    fn translate_x_unmap_event(&self, e: &mut x::XUnmapEvent) -> TranslateEventResult {
        let &mut x::XUnmapEvent {
            type_: _, serial: _, send_event: _, display: _, event: _, window,
            from_configure: _,
        } = e;
        Ok(Event::WindowHidden { window: WindowHandle(window) })
    }
    fn translate_x_visibility_event(&self, e: &mut x::XVisibilityEvent) -> TranslateEventResult {
        let &mut x::XVisibilityEvent {
            type_: _, serial: _, send_event: _, display: _, window, state,
        } = e;
        let _window = WindowHandle(window);
        match state {
            x::VisibilityUnobscured => discard_event(),
            x::VisibilityPartiallyObscured => discard_event(),
            x::VisibilityFullyObscured => discard_event(),
            _ => unreachable!{},
        }
    }

    fn translate_x_motion_event(&self, e: &mut x::XMotionEvent) -> TranslateEventResult {
        let &mut x::XMotionEvent {
            type_: _, serial: _, send_event: _, display: _, window, root: _, subwindow: _,
            time, x, y, x_root, y_root, state: _, is_hint: _, same_screen: _,
        } = e;
        let e = Event::MouseMotion {
            mouse: self.core_x_mouse(),
            timestamp: Timestamp::from_millis(time as _),
            window: WindowHandle(window),
            position: Vec2::new(x as _, y as _),
            root_position: Vec2::new(x_root as _, y_root as _),
        };
        Ok(e)
    }
    fn translate_x_crossing_event(&self, e: &mut x::XCrossingEvent) -> TranslateEventResult {
        let &mut x::XCrossingEvent {
            type_, serial: _, send_event: _, display: _, window, root: _, subwindow: _,
            time, x, y, x_root, y_root, mode, detail: _, same_screen: _, focus, state: _,
        } = e;
        let mouse = self.core_x_mouse();
        let window = WindowHandle(window);
        let timestamp = Timestamp::from_millis(time as _);
        let position = Vec2::new(x as f64, y as _);
        let root_position = Vec2::new(x_root as f64, y_root as _);
        let is_focused = focus == x::True;
        let was_focused = is_focused;
        let (is_grabbed, was_grabbed) = match mode {
            x::NotifyNormal => (false, false),
            x::NotifyGrab => (true, false),
            x::NotifyUngrab => (false, true),
            _ => unreachable!{},
        };
        let e = match type_ {
            x::EnterNotify => Event::MouseEnter {
                mouse, window, timestamp, position, root_position, is_grabbed, is_focused,
            },
            x::LeaveNotify => Event::MouseLeave {
                mouse, window, timestamp, position, root_position, was_grabbed, was_focused,
            },
            _ => unreachable!{},
        };
        Ok(e)
    }
    fn translate_x_focus_change_event(&self, e: &mut x::XFocusChangeEvent) -> TranslateEventResult {
        let &mut x::XFocusChangeEvent {
            type_, serial: _, send_event: _, display: _, window, mode: _, detail: _,
        } = e;
        let keyboard = self.core_x_keyboard();
        let window = WindowHandle(window);
        let e = match type_ {
            x::FocusIn => Event::KeyboardFocusGained {
                keyboard, window,
            },
            x::FocusOut => Event::KeyboardFocusLost {
                keyboard, window,
            },
            _ => unreachable!{},
        };
        Ok(e)
    }
    fn translate_x_expose_event(&self, e: &mut x::XExposeEvent) -> TranslateEventResult {
        let &mut x::XExposeEvent {
            type_: _, serial: _, send_event: _, display: _, window,
            x, y, width, height, count,
        } = e;
        let e = Event::WindowNeedsRedraw {
            window: WindowHandle(window),
            zone: Rect {
                x: x as _,
                y: y as _,
                w: width as _,
                h: height as _,
            },
            more_to_follow: count as _,
        };
        Ok(e)
    }
    fn translate_x_gravity_event(&self, e: &mut x::XGravityEvent) -> TranslateEventResult {
        // Window moved because its parent's position or size changed.
        // x and y are relative to the parent window's top-left corner.
        let &mut x::XGravityEvent {
            type_: _, serial: _, send_event, display: _, event: _, window, x, y,
        } = e;
        // NOTE: This is only valid as long as the only parent of this window is the root.
        let e = Event::WindowMoved {
            window: WindowHandle(window),
            position: Vec2::new(x as _, y as _),
            by_user: send_event == x::False,
        };
        Ok(e)
    }
    fn translate_x_configure_event(&self, e: &mut x::XConfigureEvent) -> TranslateEventResult {
        let &mut x::XConfigureEvent {
            type_: _, serial: _, send_event, display: _, event: _, window, x, y,
            width, height, border_width: _, above: _, override_redirect: _,
        } = e;
        let window = WindowHandle(window);
        let by_user = send_event == x::False;
        let position = Vec2::new(x as _, y as _);
        let size = Extent2::new(width as _, height as _);
        let e = Event::WindowResized {
            window, size, by_user,
        };
        self.pending_translated_events.borrow_mut().push_back(Event::WindowMoved {
            window, position, by_user,
        });
        Ok(e)
    }
    fn translate_x_mapping_event(&self, e: &mut x::XMappingEvent) -> TranslateEventResult {
        unsafe {
            x::XRefreshKeyboardMapping(e);
        }
        discard_event()
    }
    fn translate_x_client_message_event(&self, e: &mut x::XClientMessageEvent) -> TranslateEventResult {
        let &mut x::XClientMessageEvent {
            type_: _, serial: _, send_event: _, display: _, window,
            message_type, format, data,
        } = e;
        if message_type != self.atoms.WM_PROTOCOLS().unwrap() {
            return discard_event();
        }
        if format != 32 {
            return discard_event();
        }
        if data.get_long(0) == self.atoms.WM_DELETE_WINDOW().unwrap() as _ {
            let window = WindowHandle(window);
            return Ok(Event::WindowCloseRequested { window });
        }
        if let Ok(net_wm_ping) = self.atoms._NET_WM_PING() {
            if data.get_long(0) == net_wm_ping as _ {
                trace!("Replying to _NET_WM_PING (X Window {})", window);
                let reply = &mut e.clone();
                reply.serial = 0;
                reply.send_event = x::True;
                reply.window = self.x_default_root_window();
                unsafe {
                    // BadValue, BadWindow
                    x::XSendEvent(
                        self.x_display, window, x::False, 
                        x::SubstructureNotifyMask | x::SubstructureRedirectMask,
                        reply as *mut _ as _
                    );
                }
                return discard_event(); // We handled it but we have no equivalent in our API.
            }
        }
        bad_event(format!("Unhandled ClientMessage event: {:?}", e))
    }

    fn translate_x_key_event(&self, e: &mut x::XKeyEvent) -> TranslateEventResult {
        let &mut x::XKeyEvent {
            type_, serial: _, send_event: _, display: _, window, root: _, subwindow: _,
            time, x, y, x_root, y_root, state: _, keycode, same_screen: _,
        } = e;

        self.set_net_wm_user_time_for_x_window(window, time);

        let keyboard = self.core_x_keyboard();
        let window = WindowHandle(window);
        let timestamp = Timestamp::from_millis(time);

        self.pending_translated_events.borrow_mut().push_back({
            Event::MouseMotion {
                mouse: self.core_x_mouse(),
                timestamp,
                window,
                position: Vec2::new(x as _, y as _),
                root_position: Vec2::new(x_root as _, y_root as _),
            }
        });

        let keycode = keycode as x::KeyCode;

        let index_into_x_keysyms_list = 0;
        let (keysym, text) = match type_ {
            x::KeyRelease => (self.x_key_event_keysym(e, index_into_x_keysyms_list), None),
            x::KeyPress => match self.retrieve_window(window.0) {
                Err(_) => (self.x_key_event_keysym(e, index_into_x_keysyms_list), None),
                Ok(w) => match w.xic {
                    None => (self.x_key_event_keysym(e, index_into_x_keysyms_list), None),
                    Some(xic) => self.x_utf8_lookup_string(xic, e),
                },
            },
            _ => unreachable!{}
        };

        let key = Key {
            code: Keycode(keycode),
            sym: keysym.map(Keysym::from_x_keysym),
        };

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
                    x::False == x::XFilterEvent(e as *mut _ as _, 0)
                };
                let text = if is_text { text } else { None };
                Event::KeyboardKeyPressed {
                    keyboard, window, timestamp, key, is_repeat, text,
                }
            },
            _ => unreachable!{},
        };
        Ok(e)
    }

    fn translate_x_button_event(&self, e: &mut x::XButtonEvent) -> TranslateEventResult {
        let &mut x::XButtonEvent {
            type_, serial: _, send_event: _, display: _, window, root: _, subwindow: _,
            time, x, y, x_root, y_root, state: _, button, same_screen: _,
        } = e;

        self.set_net_wm_user_time_for_x_window(window, time);

        let mouse = self.core_x_mouse();
        let window = WindowHandle(window);
        let timestamp = Timestamp::from_millis(time);
        let position = Vec2::new(x as _, y as _);
        let root_position = Vec2::new(x_root as _, y_root as _);

        // http://xahlee.info/linux/linux_x11_mouse_button_number.html
        // On my R.A.T 7, 10 is right scroll and 11 is left scroll (using thumb barrel).
        // Pretty sure it's not standard though.
        let (button, scroll) = match button {
            1 => (Some(MouseButton::Left), None),
            2 => (Some(MouseButton::Middle), None),
            3 => (Some(MouseButton::Right), None),
            4 => (None, Some(Vec2::new(0,  1))),
            5 => (None, Some(Vec2::new(0, -1))),
            6 => (None, Some(Vec2::new(-1, 0))),
            7 => (None, Some(Vec2::new( 1, 0))),
            8 => (Some(MouseButton::Back), None),
            9 => (Some(MouseButton::Forward), None),
            b => (Some(MouseButton::Other(b as _)), None),
        };
        match scroll {
            Some(scroll) => match type_ {
                x::ButtonPress => Ok(Event::MouseScroll {
                    mouse, window, timestamp, position, root_position, scroll,
                }),
                x::ButtonRelease => discard_event(),
                _ => unreachable!{},
            },
            None => {
                let button = button.unwrap();
                let e = match type_ {
                    x::ButtonPress => Event::MouseButtonPressed {
                        mouse, window, timestamp, position, root_position, button, clicks: None,
                    },
                    x::ButtonRelease => Event::MouseButtonReleased {
                        mouse, window, timestamp, position, root_position, button,
                    },
                    _ => unreachable!{},
                };
                Ok(e)
            }
        }
    }

    fn translate_xi_device_changed_event(&self, e: &mut xi2::XIDeviceChangedEvent) -> TranslateEventResult {
        let &mut xi2::XIDeviceChangedEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, sourceid, reason, num_classes, classes,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_hierarchy_event(&self, e: &mut xi2::XIHierarchyEvent) -> TranslateEventResult {
        let &mut xi2::XIHierarchyEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, flags, num_info, info,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_enter_event(&self, e: &mut xi2::XIEnterEvent) -> TranslateEventResult {
        let &mut xi2::XIEnterEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, sourceid, detail, root, event: x_window, child,
            root_x, root_y,
            event_x, event_y,
            mode, focus, same_screen, buttons, mods, group,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_property_event(&self, e: &mut xi2::XIPropertyEvent) -> TranslateEventResult {
        let &mut xi2::XIPropertyEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, property, what,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_device_event(&self, e: &mut xi2::XIDeviceEvent) -> TranslateEventResult {
        let &mut xi2::XIDeviceEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, sourceid, detail, root,
            event: x_window, child, root_x, root_y,
            event_x, event_y, flags, buttons, valuators, mods, group,
        } = e;
        unimplemented!{}
    }
    fn translate_xi_raw_event(&self, e: &mut xi2::XIRawEvent) -> TranslateEventResult {
        let &mut xi2::XIRawEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time, deviceid, sourceid, detail, flags, valuators, raw_values,
        } = e;
        unimplemented!{}
    }



    fn x_key_event_keysym(&self, x_event: &mut x::XKeyEvent, index: c_int) -> Option<x::KeySym> {
        match unsafe { x::XLookupKeysym(x_event, index) } {
            x if x == x::NoSymbol as _ => None,
            x => Some(x),
        }
    }

    fn x_utf8_lookup_string(&self, xic: x::XIC, x_event: &mut x::XKeyEvent) -> (Option<x::KeySym>, Option<String>) {
        // Asserting because of undefined behaviour otherwise.
        assert_eq!(x_event.type_, x::KeyPress);
        unsafe {
            let mut buf = Vec::<u8>::with_capacity(32);
            let mut keysym: x::KeySym = 0;
            let mut status: x::Status = 0;
            loop {
                let actual_len = x::Xutf8LookupString(
                    xic, x_event,
                    buf.as_mut_ptr() as _, buf.capacity() as _,
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
    fn retrieve_window(&self, window: x::Window) -> Result<Rc<X11SharedWindow>> {
        let result = match self.weak_windows.borrow().get(&window) {
            None => failed(format!("X Window {} is not in our list", window)),
            Some(weak) => match weak.upgrade() {
                None => failed(format!("X Window {} should have been removed from the list", window)),
                Some(window) => Ok(window),
            },
        };
        if let Err(ref err) = result.as_ref() {
            warn!("Could not retrieve internal window: {}", err);
        }
        result
    }
    fn set_net_wm_user_time_for_x_window(&self, window: x::Window, time: x::Time) {
        let err = match self.retrieve_window(window) {
            Ok(w) => w.set_net_wm_user_time(time).err(),
            Err(e) => Some(e),
        };
        if let Some(e) = err {
            trace!("Could not set _NET_WM_USER_TIME for X Window {}: {}", window, e);
        } else {
            trace!("Sucessfully set _NET_WM_USER_TIME to {} for X Window {}", time, window);
        }
    }
    fn core_x_mouse(&self) -> HidID {
        HidID(X11DeviceID::CorePointer.into())
    }
    fn core_x_keyboard(&self) -> HidID {
        HidID(X11DeviceID::CoreKeyboard.into())
    }
}
