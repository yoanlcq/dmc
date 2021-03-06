use std::mem;
use std::ptr;
use std::slice;
use std::rc::Rc;
use std::os::raw::c_int;
use std::collections::HashMap;
use super::context::{X11SharedContext};
use super::x11::xlib as x;
use super::x11::xinput2 as xi2;
use super::{X11SharedWindow, X11DeviceID};
use super::device::{
    XI2DeviceCache,
    XI2DeviceRole, XI2DeviceAnyClassInfo,
    XI2ButtonLabel, XI2AxisLabel,
    XI2ValuatorClassInfo,
};
use os::{OsEventInstant};
use error::{Result, failed};
use event::{Event, EventInstant, UnprocessedEvent};
use device::{self, DeviceID, DeviceInfo, MouseButton, Key, Keysym, Keycode};
use window::WindowHandle;
use {Vec2, Extent2, Rect};

macro_rules! define_x11_unprocessed_event_enum {
    ($($Variant:ident ($Ty:ty),)+) => {
        #[derive(Debug, Clone)]
        pub enum X11UnprocessedEvent {
            $($Variant($Ty),)+
        }

        $(impl From<$Ty> for X11UnprocessedEvent {
            fn from(e: $Ty) -> Self {
                X11UnprocessedEvent::$Variant(e)
            }
        })+

        impl PartialEq for X11UnprocessedEvent {
            fn eq(&self, other: &Self) -> bool {
                unsafe {
                    let a = slice::from_raw_parts(self  as *const _ as *const u8, mem::size_of_val(self ));
                    let b = slice::from_raw_parts(other as *const _ as *const u8, mem::size_of_val(other));
                    a == b
                }
            }
        }
    };
}

define_x11_unprocessed_event_enum!{
    XEvent                (x::XEvent),
    XIBarrierEvent        (xi2::XIBarrierEvent),
    XIDeviceChangedEvent  (xi2::XIDeviceChangedEvent),
    XIDeviceEvent         (xi2::XIDeviceEvent),
    XIEnterEvent          (xi2::XIEnterEvent),
    XIEvent               (xi2::XIEvent),
    XIHierarchyEvent      (xi2::XIHierarchyEvent),
    XIPropertyEvent       (xi2::XIPropertyEvent),
    XIRawEvent            (xi2::XIRawEvent),
    XITouchOwnershipEvent (xi2::XITouchOwnershipEvent),
}

// TODO: Move this somewhere else or get rid of it
fn io_read_ready(fd: c_int) -> bool {
    loop {
        use nix::poll::*;
        let info = PollFd::new(fd, EventFlags::POLLIN | EventFlags::POLLPRI);
        match poll(&mut [info], 0 /* timeout_ms */) {
            Err(::nix::Error::Sys(::nix::errno::Errno::EINTR)) => continue,
            Ok(n) => return n > 0,
            _ => return false,
        };
    }
}

impl X11SharedContext {
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        self.xi()?;
        Ok(true)
    }

    pub fn poll_next_event(&self) -> Option<Event> {
        let ev = self.poll_next_event_real();
        let q = self.pending_translated_events.borrow();
        if let Some(ev) = ev.as_ref() {
            trace!("[EV] ---- ({} / {}): {:?}", q.len(), q.capacity(), ev);
        }
        ev
    }
    pub fn poll_next_event_real(&self) -> Option<Event> {
        let ev = self.pending_translated_events.borrow_mut().pop_front();
        if let Some(ev) = ev {
            return Some(ev);
        }
        loop {
            let n = self.x_pending();
            if n <= 0 {
                break;
            }
            for _ in 0..n {
                self.pump_x_event(&mut self.x_next_event());
            }
        }
        self.pending_translated_events.borrow_mut().pop_front()
    }
    fn x_pending(&self) -> c_int {
        let x_display = self.lock_x_display();
        unsafe {
            return x::XPending(*x_display);
            /*
            match x::XEventsQueued(*x_display, missing_bits::x::QueuedAfterFlush) {
                x if x <= 0 => (),
                n => return n,
            };
            if io_read_ready(x::XConnectionNumber(*x_display)) {
                return x::XPending(*x_display);
            }
            */
        }
        // 0
    }
    fn x_next_event(&self) -> x::XEvent {
        unsafe {
            let mut x_event = mem::zeroed();
            x::XNextEvent(*self.lock_x_display(), &mut x_event);
            x_event
        }
    }

    fn push_event(&self, e: Event) {
        trace!("[EV] ++++ ({} / {}): {:?}", self.pending_translated_events.borrow().len(), self.pending_translated_events.borrow().capacity(), e);
        self.pending_translated_events.borrow_mut().push_back(e);
    }
    // FIXME: So what should be do about these?
    fn push_unhandled_x_event<T: Into<x::XEvent>>(&self, e: T) {
        // self.push_event(Event::UnprocessedEvent(UnprocessedEvent { os_event: X11UnprocessedEvent::from(e.into()).into(), following: 0, was_ignored: true, }))
    }
    fn push_handled_x_event<T: Into<x::XEvent>>(&self, e: T, following: usize) {
        // self.push_event(Event::UnprocessedEvent(UnprocessedEvent { os_event: X11UnprocessedEvent::from(e.into()).into(), following, was_ignored: false, }))
    }
    fn push_unhandled_xi2_event<T: Into<X11UnprocessedEvent>>(&self, e: T) {
        // self.push_event(Event::UnprocessedEvent(UnprocessedEvent { os_event: e.into().into(), following: 0, was_ignored: true, }))
    }
    fn push_handled_xi2_event<T: Into<X11UnprocessedEvent>>(&self, e: T, following: usize) {
        // self.push_event(Event::UnprocessedEvent(UnprocessedEvent { os_event: e.into().into(), following, was_ignored: false, }))
    }


    fn pump_x_event(&self, e: &mut x::XEvent) {
        match e.get_type() {
            x::GenericEvent => {
                let x_display = self.lock_x_display();
                let mut cookie = x::XGenericEventCookie::from(&*e);
                unsafe {
                    if x::XGetEventData(*x_display, &mut cookie) == x::True {
                        if let Ok(xi) = self.xi() {
                            if cookie.type_ == x::GenericEvent && cookie.extension == xi.major_opcode {
                                self.pump_xi_event(&mut *(cookie.data as *mut xi2::XIEvent));
                                x::XFreeEventData(*x_display, &mut cookie);
                                return;
                            }
                        }
                    }
                    // NOTE: Yes, do it even if XGetEventData() failed! See the man page.
                    x::XFreeEventData(*x_display, &mut cookie); 
                }
                self.push_unhandled_x_event(*e);
            },
            // These events are the older couterparts to XI2 events; they don't give as much information.
            // In fact, if we were able to call XISelectEvents, we'll actually receive
            // the XI2 events instead of these.
            x::KeyPress | x::KeyRelease => self.pump_x_key_event(e.as_mut()),
            x::ButtonPress | x::ButtonRelease => self.pump_x_button_event(e.as_mut()),
            x::MotionNotify => self.pump_x_motion_event(e.as_mut()),
            x::EnterNotify | x::LeaveNotify => self.pump_x_crossing_event(e.as_mut()),
            x::FocusIn | x::FocusOut => self.pump_x_focus_change_event(e.as_mut()),
            // ---
            // ---
            x::ClientMessage => self.pump_x_client_message_event(e.as_mut()),
            x::GravityNotify => self.pump_x_gravity_event(e.as_mut()),
            x::ConfigureNotify => self.pump_x_configure_event(e.as_mut()),
            x::ResizeRequest => self.pump_x_resize_request_event(e.as_mut()),
            x::MappingNotify => self.pump_x_mapping_event(e.as_mut()),
            x::Expose  => self.pump_x_expose_event(e.as_mut()),
            x::VisibilityNotify => self.pump_x_visibility_event(e.as_mut()),
            x::MapNotify => self.pump_x_map_event(e.as_mut()),
            x::UnmapNotify => self.pump_x_unmap_event(e.as_mut()),
            // ---
            // Events that we definitely want to ignore (AFAIK)
            x::GraphicsExpose
            | x::NoExpose
            | x::ReparentNotify  
            | x::ColormapNotify  
                => self.push_unhandled_x_event(*e),
            // ---
            // Events that we're ignoring today, but might be interesting later
            x::KeymapNotify 
            | x::PropertyNotify  
            | x::CirculateRequest
            | x::ConfigureRequest
            | x::MapRequest
            | x::CirculateNotify
            | x::CreateNotify
            | x::DestroyNotify
            | x::SelectionClear  
            | x::SelectionNotify 
            | x::SelectionRequest
                => self.push_unhandled_x_event(*e),
            // ---
            // Events that we seemingly don't know about
            _   => self.push_unhandled_x_event(*e),
        }
    }

    fn pump_xi_event(&self, e: &mut xi2::XIEvent) {
        match e.evtype {
            xi2::XI_DeviceChanged => self.pump_xi_device_changed_event(unsafe { mem::transmute(e) }),
            xi2::XI_HierarchyChanged => self.pump_xi_hierarchy_event(unsafe { mem::transmute(e) }),
            xi2::XI_PropertyEvent => self.pump_xi_property_event(unsafe { mem::transmute(e) }),
              xi2::XI_Enter
            | xi2::XI_Leave
            | xi2::XI_FocusIn
            | xi2::XI_FocusOut
                => self.pump_xi_enter_event(unsafe { mem::transmute(e) }),
              xi2::XI_KeyPress
            | xi2::XI_KeyRelease
            | xi2::XI_ButtonPress
            | xi2::XI_ButtonRelease
            | xi2::XI_Motion
            | xi2::XI_TouchBegin
            | xi2::XI_TouchUpdate
            | xi2::XI_TouchEnd
                => self.pump_xi_device_event(unsafe { mem::transmute(e) }),
              xi2::XI_RawKeyPress     
            | xi2::XI_RawKeyRelease   
            | xi2::XI_RawButtonPress  
            | xi2::XI_RawButtonRelease
            | xi2::XI_RawMotion       
            | xi2::XI_RawTouchBegin   
            | xi2::XI_RawTouchUpdate  
            | xi2::XI_RawTouchEnd 
                => self.pump_xi_raw_event(unsafe { mem::transmute(e) }),
            _   => self.push_unhandled_xi2_event(*e),
        }
    }

    fn pump_x_map_event(&self, e: &mut x::XMapEvent) {
        let &mut x::XMapEvent {
            type_: _, serial: _, send_event: _, display: _, event: _, window,
            override_redirect: _,
        } = e;
        self.push_handled_x_event(*e, 1);
        self.push_event(Event::WindowShown { window: WindowHandle(window) })
    }
    fn pump_x_unmap_event(&self, e: &mut x::XUnmapEvent) {
        let &mut x::XUnmapEvent {
            type_: _, serial: _, send_event: _, display: _, event: _, window,
            from_configure: _,
        } = e;
        self.push_handled_x_event(*e, 1);
        self.push_event(Event::WindowHidden { window: WindowHandle(window) })
    }
    fn pump_x_visibility_event(&self, e: &mut x::XVisibilityEvent) {
        let &mut x::XVisibilityEvent {
            type_: _, serial: _, send_event: _, display: _, window, state,
        } = e;
        let _window = WindowHandle(window);
        match state {
            x::VisibilityUnobscured => self.push_unhandled_x_event(*e),
            x::VisibilityPartiallyObscured => self.push_unhandled_x_event(*e),
            x::VisibilityFullyObscured => self.push_unhandled_x_event(*e),
            _ => self.push_unhandled_x_event(*e),
        }
    }

    fn pump_x_motion_event(&self, e: &mut x::XMotionEvent) {
        let &mut x::XMotionEvent {
            type_: _, serial: _, send_event: _, display: _, window, root: _, subwindow: _,
            time, x, y, x_root, y_root, state: _, is_hint: _, same_screen: _,
        } = e;
        let position = Vec2::new(x as _, y as _);
        let ev = Event::MouseMotion {
            mouse: self.core_x_mouse_deviceid(),
            instant: EventInstant(OsEventInstant::X11EventTimeMillis(time)),
            window: WindowHandle(window),
            position,
            root_position: Vec2::new(x_root as _, y_root as _),
        };
        self.previous_mouse_position.set(Some(position));
        self.push_handled_x_event(*e, 1);
        self.push_event(ev)
    }
    fn pump_x_crossing_event(&self, e: &mut x::XCrossingEvent) {
        let &mut x::XCrossingEvent {
            type_, serial: _, send_event: _, display: _, window, root: _, subwindow: _,
            time, x, y, x_root, y_root, mode, detail: _, same_screen: _, focus, state: _,
        } = e;

        if self.xi().is_ok() {
            return self.push_handled_x_event(*e, 0);
        }

        let mouse = self.core_x_mouse_deviceid();
        let window = WindowHandle(window);
        let instant = EventInstant(OsEventInstant::X11EventTimeMillis(time));
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
        let motion = Event::MouseMotion { mouse, window, instant, position, root_position };
        self.previous_mouse_position.set(Some(position));
        let ev = match type_ {
            x::EnterNotify => Event::MouseEnter { mouse, window, instant, is_grabbed, is_focused },
            x::LeaveNotify => Event::MouseLeave { mouse, window, instant, was_grabbed, was_focused },
            _ => unreachable!{},
        };
        self.push_handled_x_event(*e, 2);
        self.push_event(motion);
        self.push_event(ev)
    }
    fn pump_x_focus_change_event(&self, e: &mut x::XFocusChangeEvent) {
        let &mut x::XFocusChangeEvent {
            type_, serial: _, send_event: _, display: _, window, mode: _, detail: _,
        } = e;
        let keyboard = self.core_x_keyboard_deviceid();
        let window = WindowHandle(window);
        let ev = match type_ {
            x::FocusIn => Event::KeyboardFocusGained { keyboard, window, },
            x::FocusOut => Event::KeyboardFocusLost { keyboard, window, },
            _ => unreachable!{},
        };
        self.push_handled_x_event(*e, 1);
        self.push_event(ev)
    }
    fn pump_x_expose_event(&self, e: &mut x::XExposeEvent) {
        let &mut x::XExposeEvent {
            type_: _, serial: _, send_event: _, display: _, window,
            x, y, width, height, count,
        } = e;
        let ev = Event::WindowNeedsRedraw {
            window: WindowHandle(window),
            zone: Rect {
                x: x as _,
                y: y as _,
                w: width as _,
                h: height as _,
            },
            more_to_follow: count as _,
        };
        self.push_handled_x_event(*e, 1);
        self.push_event(ev)
    }
    fn pump_x_gravity_event(&self, e: &mut x::XGravityEvent) {
        // Blah! Don't handle these; they're redundant with XConfigureEvent.
        /*
        // Window moved because its parent's position or size changed.
        // x and y are relative to the parent window's top-left corner.
        let &mut x::XGravityEvent {
            type_: _, serial: _, send_event, display: _, event: _, window, x, y,
        } = e;
        // NOTE: This is only valid as long as the only parent of this window is the root.
        let ev = Event::WindowMoved {
            window: WindowHandle(window),
            position: Vec2::new(x as _, y as _),
            by_user: send_event == x::False,
        };
        self.push_handled_x_event(*e, 1);
        self.push_event(ev)
        */
    }
    fn pump_x_configure_event(&self, e: &mut x::XConfigureEvent) {
        let &mut x::XConfigureEvent {
            type_: _, serial: _, send_event, display: _, event: _, window: x_window,
            mut x, mut y, width, height, border_width: _, above: _, override_redirect: _,
        } = e;

        if send_event == x::False {
            let mut children = ptr::null_mut();
            let mut nb_children = 0;
            let mut child = 0;
            let mut root = 0;
            let mut parent = 0;
            let x_display = self.lock_x_display();
            unsafe {
                x::XQueryTree(*x_display, x_window, &mut root, &mut parent, &mut children, &mut nb_children);
                x::XTranslateCoordinates(*x_display, parent, root, x, y, &mut x, &mut y, &mut child);
            }
        }

        let window = WindowHandle(x_window);
        let by_user = send_event == x::False;
        let position = Vec2::new(x as _, y as _);
        let size = Extent2::new(width as _, height as _);

        let w = self.weak_windows.borrow()[&x_window].upgrade().unwrap();

        if position != w.prev_pos.get() {
            if send_event != x::False {
                self.push_event(Event::WindowMoved { window, position, by_user, });
            }
            w.prev_pos.set(position);
        }
        if size != w.prev_size.get() {
            self.push_event(Event::WindowResized { window, size, by_user, });
            w.prev_size.set(size);
        }
        // self.push_handled_x_event(*e, 2); FIXME
    }
    fn pump_x_resize_request_event(&self, e: &mut x::XResizeRequestEvent) {
        unimplemented!{} // They're evil, we never use them
        /*
        let &mut x::XResizeRequestEvent {
            type_: _, serial: _, send_event, display: _, window,
            width, height,
        } = e;
        let window = WindowHandle(window);
        let by_user = send_event == x::False;
        let size = Extent2::new(width as _, height as _);

        self.push_handled_x_event(*e, 1);
        self.push_event(Event::WindowResized { window, size, by_user, })
        */
    }

    fn pump_x_mapping_event(&self, e: &mut x::XMappingEvent) {
        unsafe {
            x::XRefreshKeyboardMapping(e);
        }
        self.push_handled_x_event(*e, 0);
    }
    fn pump_x_client_message_event(&self, e: &mut x::XClientMessageEvent) {
        let x_display = self.lock_x_display();
        let &mut x::XClientMessageEvent {
            type_: _, serial: _, send_event: _, display: _, window,
            message_type, format, data,
        } = e;
        if message_type != self.atoms.WM_PROTOCOLS().unwrap() {
            return self.push_unhandled_x_event(&*e);
        }
        if format != 32 {
            return self.push_unhandled_x_event(&*e);
        }
        if data.get_long(0) == self.atoms.WM_DELETE_WINDOW().unwrap() as _ {
            let window = WindowHandle(window);
            self.push_handled_x_event(&*e, 1);
            return self.push_event(Event::WindowCloseRequested { window });
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
                        *x_display, window, x::False, 
                        x::SubstructureNotifyMask | x::SubstructureRedirectMask,
                        reply as *mut _ as _
                    );
                }
                return self.push_handled_x_event(&*e, 0);
            }
        }
        self.push_unhandled_x_event(&*e)
    }

    fn pump_x_key_event(&self, e: &mut x::XKeyEvent) {
        // First of all, check if it is a key repeat event.
        // We detect this when a KeyRelease event is diretcly followed by a KeyPress, both having the exact same time.
        let mut is_repeat = false;
        if e.type_ == x::KeyRelease && self.x_pending() > 0 {
            let x_display = self.lock_x_display();

            // Peek (don't remove) event from the queue.
            let mut next_ev = unsafe { 
                let mut next_ev = mem::zeroed();
                x::XPeekEvent(*x_display, &mut next_ev);
                next_ev
            };
            if unsafe { next_ev.type_ } == x::KeyPress {
                let next_ev: &mut x::XKeyEvent = next_ev.as_mut();

                if next_ev.time == e.time && next_ev.keycode == e.keycode {
                    is_repeat = true;
                    // We've proved it as a key repeat. We're handling this ourselves, so remove
                    // it from the queue.
                    unsafe {
                        x::XNextEvent(*x_display, next_ev as *mut _ as _);
                    }
                    *e = *next_ev;
                }
            }
        }

        let &mut x::XKeyEvent {
            type_, serial: _, send_event: _, display: _, window, root: _, subwindow: _,
            time, x, y, x_root, y_root, state: _, keycode, same_screen: _,
        } = e;

        self.set_net_wm_user_time_for_x_window(window, time);


        let keyboard = {
            // The raw counterpart to a KeyEvent is always sent right before it, with the
            // same timestamp. We can use this to find the keyboard's actual ID, since
            // we don't use XI_Key* events, for reasons (see src/x11/xi2.rs).
            let (sourceid, xi_time, xi_keycode) = self.previous_xi_raw_key_event.get();
            if time == xi_time && xi_keycode == keycode as _ {
                DeviceID(X11DeviceID::XISlave(sourceid).into())
            } else {
                self.core_x_keyboard_deviceid()
            }
        };
        let window = WindowHandle(window);
        let instant = EventInstant(OsEventInstant::X11EventTimeMillis(time));
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

        let position = Vec2::new(x as _, y as _);
        let mouse_ev = if self.previous_mouse_position.replace(Some(position)) == Some(position) {
            None
        } else {
            Some(Event::MouseMotion {
                mouse: self.core_x_mouse_deviceid(),
                instant,
                window,
                position,
                root_position: Vec2::new(x_root as _, y_root as _),
            })
        };

        let repeat_count = 1;

        let key_ev = match type_ {
            x::KeyRelease => Event::KeyboardKeyReleased { keyboard, window, instant, key },
            x::KeyPress => Event::KeyboardKeyPressed { keyboard, window, instant, key, is_repeat, repeat_count },
            _ => unreachable!{},
        };
        let key_ev = if keycode == 0 { None } else { Some(key_ev) };

        let text_ev = if type_ == x::KeyPress {
            let is_text = unsafe {
                x::False == x::XFilterEvent(e as *mut _ as _, 0)
            };
            if is_text {
                if let Some(text) = text { // Should be unwrap(), but being careful doesn't hurt I guess
                    Some(Event::KeyboardTextString { keyboard, window, instant, is_repeat, repeat_count, text})
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let nb_events = mouse_ev.is_some() as usize + key_ev.is_some() as usize + text_ev.is_some() as usize;
        if nb_events > 0 {
            self.push_handled_x_event(&*e, nb_events);
            if let Some(mouse_ev) = mouse_ev {
                self.push_event(mouse_ev);
            }
            if let Some(key_ev) = key_ev {
                self.push_event(key_ev);
            }
            if let Some(text_ev) = text_ev {
                self.push_event(text_ev);
            }
        }
    }

    fn x11_button_to_mousebutton_or_scroll(button: u32) -> (Option<MouseButton>, Option<Vec2<i32>>) {
        // http://xahlee.info/linux/linux_x11_mouse_button_number.html
        // On my R.A.T 7, 10 is right scroll and 11 is left scroll (using thumb barrel).
        // Pretty sure it's not standard though.
        match button {
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
        }
    }

    fn xi2_button_label_to_mouse_button_or_scroll(detail: c_int, label: Option<XI2ButtonLabel>) -> (Option<MouseButton>, Option<Vec2<i32>>) {
        match label {
            None | Some(XI2ButtonLabel::Unknown ) => {
                if detail >= 1 && detail <= 3 {
                    Self::x11_button_to_mousebutton_or_scroll(detail as _)
                } else {
                    (Some(MouseButton::Other(detail)), None)
                }
            },
            Some(XI2ButtonLabel::Extra          ) => (Some(MouseButton::Extra    ), None),
            Some(XI2ButtonLabel::Left           ) => (Some(MouseButton::Left     ), None),
            Some(XI2ButtonLabel::Middle         ) => (Some(MouseButton::Middle   ), None),
            Some(XI2ButtonLabel::Right          ) => (Some(MouseButton::Right    ), None),
            Some(XI2ButtonLabel::Side           ) => (Some(MouseButton::Side     ), None),
            Some(XI2ButtonLabel::Forward        ) => (Some(MouseButton::Forward  ), None),
            Some(XI2ButtonLabel::Back           ) => (Some(MouseButton::Back     ), None),
            Some(XI2ButtonLabel::Task           ) => (Some(MouseButton::Task     ), None),
            Some(XI2ButtonLabel::Other(other)   ) => (Some(MouseButton::Other(other as _)), None),
            Some(XI2ButtonLabel::WheelUp        ) => (None, Some(Vec2::new( 0,  1))),
            Some(XI2ButtonLabel::WheelDown      ) => (None, Some(Vec2::new( 0, -1))),
            Some(XI2ButtonLabel::HorizWheelLeft ) => (None, Some(Vec2::new(-1,  0))),
            Some(XI2ButtonLabel::HorizWheelRight) => (None, Some(Vec2::new( 1,  0))),
        }
    }

    fn pump_x_button_event(&self, e: &mut x::XButtonEvent) {
        let &mut x::XButtonEvent {
            type_, serial: _, send_event: _, display: _, window, root: _, subwindow: _,
            time, x, y, x_root, y_root, state: _, button, same_screen: _,
        } = e;

        self.set_net_wm_user_time_for_x_window(window, time);

        let mouse = self.core_x_mouse_deviceid();
        let window = WindowHandle(window);
        let instant = EventInstant(OsEventInstant::X11EventTimeMillis(time));
        let position = Vec2::new(x as _, y as _);
        let root_position = Vec2::new(x_root as _, y_root as _);

        let (button, scroll) = Self::x11_button_to_mousebutton_or_scroll(button);
        let motion = if self.previous_mouse_position.replace(Some(position)) == Some(position) {
            None
        } else {
            Some(Event::MouseMotion { mouse, window, instant, position, root_position })
        };

        let ev = match scroll {
            Some(scroll) => match type_ {
                x::ButtonPress => Some(Event::MouseScroll { mouse, window, instant, scroll: scroll.map(|x| x as _), }),
                x::ButtonRelease => None, // Ignore button release events when it's a scroll button
                _ => unreachable!{},
            },
            None => {
                let button = button.unwrap();
                let ev = match type_ {
                    x::ButtonPress => Event::MouseButtonPressed { mouse, window, instant, button, clicks: None },
                    x::ButtonRelease => Event::MouseButtonReleased { mouse, window, instant, button },
                    _ => unreachable!{},
                };
                Some(ev)
            },
        };
        let nb_events = motion.is_some() as usize + ev.is_some() as usize;
        if nb_events > 0 {
            self.push_handled_x_event(&*e, nb_events);
            if let Some(motion) = motion {
                self.push_event(motion);
            }
            if let Some(ev) = ev {
                self.push_event(ev);
            }
        }
    }

    fn pump_xi_device_changed_event(&self, e: &mut xi2::XIDeviceChangedEvent) {
        let &mut xi2::XIDeviceChangedEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time: _, deviceid, sourceid,
            reason, // SlaveSwitch, DeviceChange
            num_classes, classes,
        } = e;

        match reason {
            xi2::XISlaveSwitch => {
                // deviceid is a master, sourceid is the new slave
                self.xi2_devices.borrow_mut().get_mut(&deviceid).unwrap().info.attachment = sourceid;
            },
            xi2::XIDeviceChange => {
                // Use sourceid; deviceid is undefined
                // Also don't bother trying to only update the classes. We still lack info such as
                // the name, the role and even properties (we don't get PropertyAdded events).
                // Seriously, let's re-query everything and call it a day.
                let dev = unsafe {
                    super::device::refresh_xi2_device_cache(*self.lock_x_display(), sourceid, &self.atoms)
                };
                if let Ok(dev) = dev {
                    self.xi2_devices.borrow_mut().insert(sourceid, dev);
                }
            },
            _ => unreachable!(),
        }

        self.push_handled_xi2_event(*e, 0);
    }
    fn pump_xi_property_event(&self, e: &mut xi2::XIPropertyEvent) {
        let &mut xi2::XIPropertyEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time: _, 
            deviceid,
            property, // Atom
            what, // PropertyCreated, PropertyDeleted, PropertyModified
        } = e;

        match what {
            xi2::XIPropertyDeleted => {
                self.xi2_devices.borrow_mut().get_mut(&deviceid).unwrap().props.remove(&property);
            },
            xi2::XIPropertyCreated | xi2::XIPropertyModified => {
                if self.atoms.is_interesting_xi2_prop(property) {
                    let value = unsafe {
                        super::device::xi2_get_device_property(*self.lock_x_display(), deviceid, property)
                    };
                    if let Ok(Some(value)) = value {
                        self.xi2_devices.borrow_mut().get_mut(&deviceid).unwrap().props.insert(property, value);
                    }
                }
            },
            _ => unreachable!{},
        };

        self.push_handled_xi2_event(*e, 0);
    }

    fn pump_xi_hierarchy_event(&self, e: &mut xi2::XIHierarchyEvent) {
        let &mut xi2::XIHierarchyEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype: _,
            time: _,
            flags: _, // Combination of MasterAdded, MasterRemoved, SlaveAttached, SlaveDetached, SlaveAdded, SlaveRemoved, DeviceEnabled, DeviceDisabled
            num_info,
            info,
        } = e;

        let info = unsafe { slice::from_raw_parts(info, num_info as _) };

        let mut xi2_devices = self.xi2_devices.borrow_mut();

        for info in info {
            let &xi2::XIHierarchyInfo {
                deviceid, attachment, _use, enabled: _, flags,
            } = info;
            // - if type is MasterPointer or MasterKeyboard, attachment decribes the pairing of this device.
            // - if type is SlavePointer or SlaveKeyboard, attachment describes the master device this device is attached to.
            // - if type is FloatingSlave device, attachment is undefined.
            if let Some(dev) = xi2_devices.get_mut(&deviceid) {
                dev.info.role = XI2DeviceRole::try_from_xi2_use(_use);
            }

            let refresh_xi2_device_cache = || unsafe {
                super::device::refresh_xi2_device_cache(*self.lock_x_display(), deviceid, &self.atoms).unwrap()
            };
            if (flags & xi2::XIMasterAdded   ) != 0 { xi2_devices.insert(deviceid, refresh_xi2_device_cache()); }
            if (flags & xi2::XISlaveAdded    ) != 0 { xi2_devices.insert(deviceid, refresh_xi2_device_cache()); }
            if (flags & xi2::XISlaveAttached ) != 0 { xi2_devices.get_mut(&deviceid).unwrap().info.attachment = attachment; }
            if (flags & xi2::XISlaveDetached ) != 0 { xi2_devices.get_mut(&deviceid).unwrap().info.attachment = -1; }
            if (flags & xi2::XIDeviceEnabled ) != 0 { xi2_devices.get_mut(&deviceid).unwrap().info.is_enabled = true; }
            if (flags & xi2::XIDeviceDisabled) != 0 { xi2_devices.get_mut(&deviceid).unwrap().info.is_enabled = false; }
            if (flags & xi2::XIMasterRemoved ) != 0 { xi2_devices.remove(&deviceid); }
            if (flags & xi2::XISlaveRemoved  ) != 0 { xi2_devices.remove(&deviceid); }
        }

        self.push_unhandled_xi2_event(*e);
    }

    fn pump_xi_enter_event(&self, e: &mut xi2::XIEnterEvent) {
        let &mut xi2::XIEnterEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype,
            time, deviceid, sourceid, detail, root, event: x_window, child,
            root_x, root_y,
            event_x, event_y,
            mode, focus, same_screen, buttons, mods, group,
        } = e;

        if deviceid == sourceid { // Ignore master device events; To us, they're duplicates of slave device events
            return self.push_handled_xi2_event(*e, 0);
        }

        let keyboard = DeviceID(X11DeviceID::XISlave(sourceid).into());
        let mouse = keyboard; // Yes, absolutely
        let window = WindowHandle(x_window);
        let instant = EventInstant(OsEventInstant::X11EventTimeMillis(time));
        let position = Vec2::new(event_x, event_y);
        let root_position = Vec2::new(root_x, root_y);
        let is_focused = focus == x::True;
        let was_focused = is_focused;
        let motion = Event::MouseMotion { mouse, window, instant, position, root_position };
        self.previous_mouse_position.set(Some(position));
        let (is_grabbed, was_grabbed) = match mode {
            xi2::XINotifyNormal => (false, false),
            xi2::XINotifyGrab => (true, false),
            xi2::XINotifyUngrab => (false, true),
            _ => unreachable!{},
        };
        let ev = match evtype {
            xi2::XI_Enter => Event::MouseEnter { mouse, window, instant, is_grabbed, is_focused },
            xi2::XI_Leave => Event::MouseLeave { mouse, window, instant, was_grabbed, was_focused },
            xi2::XI_FocusIn => Event::KeyboardFocusGained { keyboard, window, },
            xi2::XI_FocusOut => Event::KeyboardFocusLost { keyboard, window, },
            _ => unreachable!{},
        };
        self.push_handled_xi2_event(*e, 2);
        self.push_event(motion);
        self.push_event(ev);
    }

    fn pump_xi_device_event(&self, e: &mut xi2::XIDeviceEvent) {
        let &mut xi2::XIDeviceEvent {
            _type: _, serial, send_event, display: _, extension: _, evtype,
            time, deviceid, sourceid, detail, // detail: The button number, key code, touch ID, or 0.
            root, event: x_window, child, // windows
            root_x, root_y, event_x, event_y,
            flags, // KeyRepeat, PointerEmulated, TouchPendingEnd, TouchEmulatingPointer
            buttons, valuators,
            mods, group, // XKB group and modifiers state
        } = e;

        // Ignore master device events; To us, they're duplicates of slave device events.
        // Also ignore emulated legacy events, otherwise we'll have redundancy.
        if deviceid == sourceid || (flags & xi2::XIPointerEmulated) != 0 {          
            return self.push_handled_xi2_event(*e, 0);
        }

        let instant = EventInstant(OsEventInstant::X11EventTimeMillis(time));
        let position = Vec2::new(event_x, event_y);
        let root_position = Vec2::new(root_x, root_y);
        let window = WindowHandle(x_window);
        let slave_device_id = DeviceID(X11DeviceID::XISlave(sourceid).into());
        let motion_ev = if Some(position) == self.previous_mouse_position.replace(Some(position)) {
            None
        } else {
            Some(Event::MouseMotion { mouse: slave_device_id, instant, window, position, root_position })
        };

        let valuators_mask = unsafe {
            slice::from_raw_parts(valuators.mask, valuators.mask_len as _)
        };
        let nb_values = {
            let mut nb_values = 0;
            for i in 0..valuators.mask_len*8 {
                if xi2::XIMaskIsSet(valuators_mask, i) {
                    nb_values = i+1;
                }
            }
            nb_values as usize
        };
        let valuators_values = unsafe {
            slice::from_raw_parts(valuators.values, nb_values)
        };

        let mut xi2_devices = self.xi2_devices.borrow_mut();

        match evtype {
            // Ignore XI Key events. See src/x11/xi.rs for a rationale.
            xi2::XI_KeyPress | xi2::XI_KeyRelease => self.push_unhandled_xi2_event(*e),
            xi2::XI_Motion => {
                let mut prev_abs_scroll = Vec2::new(None, None);
                let mut cur_abs_scroll = Vec2::new(None, None);

                let dev = xi2_devices.get_mut(&sourceid).unwrap();
                let mut valuator_i = 0;

                for i in 0..nb_values {
                    if !xi2::XIMaskIsSet(valuators_mask, i as _) {
                        continue;
                    }
                    let value = valuators_values[valuator_i];

                    let &mut XI2ValuatorClassInfo {
                        label, axis_info: _, value: ref mut previous_value,
                    } = dev.info.valuator_classes.get_mut(&i).unwrap();

                    let scroll_delta = dev.info.scroll_classes.get(&i).map(|x| value / x.increment as f64);

                    match label {
                        None => (),
                        // Scroll motion
                        Some(XI2AxisLabel::RelHorizScroll ) => {
                            cur_abs_scroll.x = Some(scroll_delta.unwrap());
                            prev_abs_scroll.x = Some(*previous_value);
                            *previous_value = cur_abs_scroll.x.unwrap();
                        },
                        Some(XI2AxisLabel::RelVertScroll  ) => {
                            cur_abs_scroll.y = Some(scroll_delta.unwrap());
                            prev_abs_scroll.y = Some(*previous_value);
                            *previous_value = cur_abs_scroll.y.unwrap();
                        },
                        // (Normally) regular mouse events, in which case the MouseMotion event is pushed anyway.
                        Some(XI2AxisLabel::AbsX           ) => (),
                        Some(XI2AxisLabel::AbsY           ) => (),
                        Some(XI2AxisLabel::RelX           ) => (),
                        Some(XI2AxisLabel::RelY           ) => (),
                        // I'm not handling these yet
                        Some(XI2AxisLabel::AbsMTTouchMajor) => (),
                        Some(XI2AxisLabel::AbsMTPressure  ) => (),
                        Some(XI2AxisLabel::AbsPressure    ) => (),
                        Some(XI2AxisLabel::AbsTiltX       ) => (),
                        Some(XI2AxisLabel::AbsTiltY       ) => (),
                        Some(XI2AxisLabel::AbsWheel       ) => (),
                        Some(XI2AxisLabel::Other(_)       ) => (),
                    }

                    valuator_i += 1;
                }

                let has_scroll_event = cur_abs_scroll.x.is_some() || cur_abs_scroll.y.is_some();
                let nb_events = motion_ev.is_some() as usize + has_scroll_event as usize;

                self.push_handled_xi2_event(*e, nb_events);
                if let Some(motion_ev) = motion_ev {
                    self.push_event(motion_ev);
                }
                if has_scroll_event {
                    let  cur_abs_scroll =  cur_abs_scroll.map(|x| x.unwrap_or(0.));
                    let prev_abs_scroll = prev_abs_scroll.map(|x| x.unwrap_or(0.));
                    let mut scroll = cur_abs_scroll - prev_abs_scroll;
                    scroll.y *= -1.;
                    self.push_event(Event::MouseScroll { mouse: slave_device_id, window, instant, scroll });
                }
            },
            xi2::XI_ButtonPress | xi2::XI_ButtonRelease => {
                self.set_net_wm_user_time_for_x_window(x_window, time);

                assert!(detail > 0);
                let label = xi2_devices[&sourceid].info.button_class.as_ref().unwrap().button_labels[detail as usize - 1];
                let (button, scroll) = Self::xi2_button_label_to_mouse_button_or_scroll(detail, label);
                let nb_events = motion_ev.is_some() as usize + button.is_some() as usize + scroll.is_some() as usize;
                self.push_handled_xi2_event(*e, nb_events);
                if let Some(motion_ev) = motion_ev {
                    self.push_event(motion_ev);
                }
                if let Some(button) = button {
                    let button_ev = match evtype {
                        xi2::XI_ButtonPress   => Event::MouseButtonPressed  { mouse: slave_device_id, window, instant, button, clicks: None, },
                        xi2::XI_ButtonRelease => Event::MouseButtonReleased { mouse: slave_device_id, window, instant, button },
                        _ => unreachable!(),
                    };
                    self.push_event(button_ev);
                }
                if let Some(scroll) = scroll {
                    self.push_event(Event::MouseScroll { mouse: slave_device_id, window, instant, scroll: scroll.map(|x| x as f64) });
                }
            },
            // These are to be done
            xi2::XI_TouchBegin => self.push_unhandled_xi2_event(*e),
            xi2::XI_TouchUpdate => self.push_unhandled_xi2_event(*e),
            xi2::XI_TouchEnd => self.push_unhandled_xi2_event(*e),
            _ => self.push_unhandled_xi2_event(*e),
        }
    }
    fn pump_xi_raw_event(&self, e: &mut xi2::XIRawEvent) {
        let &mut xi2::XIRawEvent {
            _type: _, serial: _, send_event: _, display: _, extension: _, evtype,
            time, deviceid, sourceid, detail,
            flags,
            valuators, // Accelerated values
            raw_values, // Unaccelerated values
        } = e;

        // Ignore master device events; To us, they're duplicates of slave device events.
        // Also ignore emulated legacy events, otherwise we'll have redundancy.
        if deviceid == sourceid || (flags & xi2::XIPointerEmulated) != 0 {
            return self.push_handled_xi2_event(*e, 0);
        }

        let instant = EventInstant(OsEventInstant::X11EventTimeMillis(time));
        let valuators_mask = unsafe {
            slice::from_raw_parts(valuators.mask, valuators.mask_len as _)
        };
        let nb_values = {
            let mut nb_values = 0;
            for i in 0..valuators.mask_len*8 {
                if xi2::XIMaskIsSet(valuators_mask, i) {
                    nb_values = i+1;
                }
            }
            nb_values as usize
        };
        let valuators_values = unsafe {
            slice::from_raw_parts(valuators.values, nb_values)
        };
        let raw_values = unsafe {
            slice::from_raw_parts(raw_values, nb_values)
        };

        let slave_device_id = DeviceID(X11DeviceID::XISlave(sourceid).into());
        let xi2_devices = self.xi2_devices.borrow();

        match evtype {
            xi2::XI_RawKeyPress | xi2::XI_RawKeyRelease => {
                let keyboard = slave_device_id;
                let keycode = detail as x::KeyCode;
                let key = Key {
                    code: Keycode(keycode),
                    sym: self.x_keycode_to_keysym(keycode, 0).map(Keysym::from_x_keysym),
                    // The code => sym translation is supposedly keyboard-specific, but I found no API in X11
                    // that allows doing this (accepting an XInput2 device id).
                };
                let ev = match evtype {
                    xi2::XI_RawKeyPress => Event::KeyboardKeyPressedRaw { keyboard, instant, key },
                    xi2::XI_RawKeyRelease => Event::KeyboardKeyReleasedRaw { keyboard, instant, key },
                    _ => unreachable!{},
                };
                self.push_handled_xi2_event(*e, 1);
                self.push_event(ev);
                self.previous_xi_raw_key_event.set((sourceid, time, keycode));
            },
            xi2::XI_RawMotion => {
                let mouse = slave_device_id;
                let mut scroll = Vec2::new(None, None);
                let mut displacement = Vec2::new(None, None);

                let dev = &xi2_devices[&sourceid];
                let mut valuator_i = 0;

                for i in 0..nb_values {
                    if !xi2::XIMaskIsSet(valuators_mask, i as _) {
                        continue;
                    }
                    let value = valuators_values[valuator_i];

                    let &XI2ValuatorClassInfo {
                        label, axis_info: _, value: _,
                    } = &dev.info.valuator_classes[&i];

                    let scroll_delta = dev.info.scroll_classes.get(&i).map(|x| value / x.increment as f64);

                    match label {
                        None => (),
                        // Scroll motion
                        Some(XI2AxisLabel::RelHorizScroll ) => scroll.x = Some(scroll_delta.unwrap()),
                        Some(XI2AxisLabel::RelVertScroll  ) => scroll.y = Some(-scroll_delta.unwrap()),
                        // Relative mouse motion
                        Some(XI2AxisLabel::RelX           ) => displacement.x = Some(value),
                        Some(XI2AxisLabel::RelY           ) => displacement.y = Some(value),
                        // I'm not handling these yet
                        Some(XI2AxisLabel::AbsX           ) => (),
                        Some(XI2AxisLabel::AbsY           ) => (),
                        Some(XI2AxisLabel::AbsMTTouchMajor) => (),
                        Some(XI2AxisLabel::AbsMTPressure  ) => (),
                        Some(XI2AxisLabel::AbsPressure    ) => (),
                        Some(XI2AxisLabel::AbsTiltX       ) => (),
                        Some(XI2AxisLabel::AbsTiltY       ) => (),
                        Some(XI2AxisLabel::AbsWheel       ) => (),
                        Some(XI2AxisLabel::Other(_)       ) => (),
                    }

                    valuator_i += 1;
                }

                let has_scroll_event = scroll.x.is_some() || scroll.y.is_some();
                let has_displacement_event = displacement.x.is_some() || displacement.y.is_some();
                let nb_events = has_displacement_event as usize + has_scroll_event as usize;

                self.push_handled_xi2_event(*e, nb_events);
                if has_displacement_event {
                    let displacement = displacement.map(|x| x.unwrap_or(0.));
                    self.push_event(Event::MouseMotionRaw { mouse, instant, displacement });
                }
                if has_scroll_event {
                    let scroll = scroll.map(|x| x.unwrap_or(0.));
                    self.push_event(Event::MouseScrollRaw { mouse, instant, scroll });
                }
            },
            xi2::XI_RawButtonPress | xi2::XI_RawButtonRelease => {
                let mouse = slave_device_id;
                assert!(detail > 0);
                let label = xi2_devices[&sourceid].info.button_class.as_ref().unwrap().button_labels[detail as usize - 1];
                let (button, scroll) = Self::xi2_button_label_to_mouse_button_or_scroll(detail, label);
                let nb_events = button.is_some() as usize + scroll.is_some() as usize;
                self.push_handled_xi2_event(*e, nb_events);
                if let Some(button) = button {
                    let button_ev = match evtype {
                        xi2::XI_RawButtonPress   => Event::MouseButtonPressedRaw  { mouse, instant, button, },
                        xi2::XI_RawButtonRelease => Event::MouseButtonReleasedRaw { mouse, instant, button },
                        _ => unreachable!(),
                    };
                    self.push_event(button_ev);
                }
                if let Some(scroll) = scroll {
                    self.push_event(Event::MouseScrollRaw { mouse, instant, scroll: scroll.map(|x| x as f64) });
                }
            },
            xi2::XI_RawTouchBegin => self.push_unhandled_xi2_event(*e),
            xi2::XI_RawTouchUpdate => self.push_unhandled_xi2_event(*e),
            xi2::XI_RawTouchEnd => self.push_unhandled_xi2_event(*e),
            _ => self.push_unhandled_xi2_event(*e),
        }
    }

    pub fn x_keycode_to_keysym(&self, keycode: x::KeyCode, index: c_int) -> Option<x::KeySym> {
        unsafe {
            match x::XKeycodeToKeysym(*self.lock_x_display(), keycode, index) {
                x if x == x::NoSymbol as _ => None,
                sym => Some(sym),
            }
        }
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
    pub fn core_x_mouse(&self) -> X11DeviceID {
        X11DeviceID::CorePointer
    }
    pub fn core_x_keyboard(&self) -> X11DeviceID {
        X11DeviceID::CoreKeyboard
    }
    pub fn core_x_mouse_deviceid(&self) -> DeviceID {
        DeviceID(self.core_x_mouse().into())
    }
    pub fn core_x_keyboard_deviceid(&self) -> DeviceID {
        DeviceID(self.core_x_keyboard().into())
    }
    pub fn devices(&self) -> device::Result<HashMap<DeviceID, DeviceInfo>> {
        // FIXME
        device::failed("This is not implemented yet, but this doesn't panic so I can test stuff")
    }
}
