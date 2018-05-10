extern crate libc;

use std::ptr;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::os::raw::{c_void, c_char, c_int, c_uint, c_long, c_ulong};
use std::ops::{Deref, Range};
use std::mem;
use std::env;
use std::ffi::CString;

use window::{self, Window, WindowSettings, WindowHandle, WindowTypeHint, WindowStyleHint};
use error::{Result, failed, failed_unexplained};
use device::{self, DeviceID, WindowMouseState, WindowTabletState};
use vek::{Vec2, Extent2, Rect, Clamp, Rgba};

use super::x11::xlib as x;
use super::{X11Context, X11SharedContext};
use super::cursor::X11Cursor;
use super::missing_bits;
use super::net_wm::{NetWMStateAction, NetWMWindowType, BypassCompositor};
use super::motif_wm;
use super::prop::{self, PropType, PropMode, PropElement, PropData};
use super::xlib_error;


pub type X11WindowHandle = x::Window;

#[derive(Debug, Clone, PartialEq)]
pub struct X11WindowFromHandleParams;

#[derive(Debug)]
pub struct X11SharedWindow {
    pub context: Rc<X11SharedContext>,
    pub x_window: x::Window,
    pub colormap: x::Colormap,
    // NOTE: If I implement child windows one day, they should not have their own XIC.
    // Or should they?
    pub xic: Option<x::XIC>,
    pub user_cursor: RefCell<Option<X11Cursor>>,
    pub is_cursor_visible: Cell<bool>,
}

#[derive(Debug)]
pub struct X11Window(pub Rc<X11SharedWindow>);


impl Window {
    /// (X11-only) Gets the X `Display` pointer associated with the `Context` for this `Window`.
    ///
    /// Be careful: It is closed when the `Context` is dropped.
    pub fn xlib_display(&self) -> *mut x::Display {
        self.0.context.x_display
    }
    /// (X11-only) Gets the X input context (XIC) associated with this `Window`, if present.
    ///
    /// Be careful: It is destroyed when the `Window` is dropped.
    pub fn xic(&self) -> Option<x::XIC> {
        self.0.xic
    }
}

impl WindowHandle {
    /// (X11-only) Gets the X Window wrapped under this `WindowHandle`.
    pub fn x_window(&self) -> x::Window {
        self.0
    }
}

impl Deref for X11Window {
    type Target = X11SharedWindow;
    fn deref(&self) -> &X11SharedWindow {
        &self.0
    }
}

impl Drop for X11SharedWindow {
    fn drop(&mut self) {
        let &mut Self {
            ref mut context, x_window, colormap, xic, user_cursor: _,
            is_cursor_visible: _,
        } = self;

        let x_display = context.x_display;

        match context.weak_windows.borrow_mut().remove(&x_window) {
            Some(_weak) => trace!("Removed X Window {} from the context's list", x_window),
            None => warn!("X Window {} is being destroyed but somehow wasn't in the context's list", x_window),
        }

        unsafe {
            if let Some(xic) = xic {
                x::XDestroyIC(xic);
                trace!("Destroyed XIC {:?}", xic);
            }
            x::XDestroyWindow(x_display, x_window);
            trace!("Destroyed X Window {}", x_window);
            x::XFreeColormap(x_display, colormap);
            trace!("Freed Colormap {}", colormap);
        }
    }
}


impl X11Context {
    pub fn create_window(&self, window_settings: &WindowSettings) -> Result<X11Window> {
        let x_display = self.x_display;

        let parent = unsafe {
            x::XDefaultRootWindow(x_display)
        };

        let &WindowSettings {
            position, size, ref opengl, high_dpi,
        } = window_settings;

        if high_dpi {
            warn!("The `high_dpi` setting was set to `true`, but will not be handled.");
        }

        let Extent2 { w, h } = size;
        let Vec2 { x, y } = position;

        let (visual, depth, colormap) = match *opengl {
            Some(ref pixel_format) => {
                unimplemented!{"We need to load the GLX extension"}
                /*
                if self.glx.is_none() {
                    return failed("Cannot create OpenGL-capable window without GLX");
                }
                let vi = unsafe { *pixel_format.0.visual_info };
                let colormap = unsafe {
                    x::XCreateColormap(x_display, parent, vi.visual, x::AllocNone)
                };
                (vi.visual, vi.depth, colormap)
                */
            },
            None => {
                let screen_num = unsafe {
                    x::XDefaultScreen(x_display)
                };
                let depth = x::CopyFromParent;
                let visual = unsafe {
                    x::XDefaultVisual(x_display, screen_num)
                };
                let colormap = match unsafe { xlib_error::sync_catch(x_display, || {
                    x::XCreateColormap(x_display, parent, visual, x::AllocNone)
                })} {
                    Ok(c) => c,
                    Err(e) => return failed(format!("XCreateColormap failed with {}", e)),
                };
                (visual, depth, colormap)
            },
        };

        let border_thickness = 0;
        let class = x::InputOutput;
        let valuemask = x::CWColormap | x::CWEventMask | x::CWBackPixel;
        let mut swa = x::XSetWindowAttributes {
            colormap,
            event_mask: { // Basically, all events. Copy-pasted from /usr/include/x11/X.h
                  x::KeyPressMask
                | x::KeyReleaseMask
                | x::ButtonPressMask
                | x::ButtonReleaseMask
                | x::EnterWindowMask
                | x::LeaveWindowMask
                | x::PointerMotionMask
                // | x::PointerMotionHintMask // But not this flag, it sucks! >:(
                | x::Button1MotionMask
                | x::Button2MotionMask
                | x::Button3MotionMask
                | x::Button4MotionMask
                | x::Button5MotionMask
                | x::ButtonMotionMask
                | x::KeymapStateMask
                | x::ExposureMask
                | x::VisibilityChangeMask
                | x::StructureNotifyMask
                | x::ResizeRedirectMask
                | x::SubstructureNotifyMask
                | x::SubstructureRedirectMask
                | x::FocusChangeMask
                | x::PropertyChangeMask
                | x::ColormapChangeMask
                | x::OwnerGrabButtonMask
            },
            background_pixmap    : 0,
            background_pixel     : unsafe {
                x::XWhitePixel(x_display, 0)
            },
            border_pixmap        : 0,
            border_pixel         : 0,
            bit_gravity          : 0,
            win_gravity          : 0,
            backing_store        : 0,
            backing_planes       : 0,
            backing_pixel        : 0,
            save_under           : 0,
            do_not_propagate_mask: 0,
            override_redirect    : 0,
            cursor               : 0,
        };

        let x_window = unsafe { xlib_error::sync_catch(x_display, || {
            x::XCreateWindow(
                x_display, parent, x, y, w, h,
                border_thickness, depth, class as _, visual, valuemask, &mut swa
            )
        })}?;
        if x_window == 0 {
            unsafe {
                x::XFreeColormap(x_display, colormap);
            }
            return failed("XCreateWindow() returned 0");
        }
        trace!("Created X Window {}", x_window);

        // We're not done: Say which protocols we support, and
        // set our process ID property for _NET_WM_PING.
        {
            let mut protocols_len = 0_usize;
            let mut protocols = [0; 3];
            if let Ok(atom) = self.atoms.WM_DELETE_WINDOW() {
                protocols[protocols_len] = atom;
                protocols_len += 1;
            }
            if let Ok(atom) = self.atoms.WM_TAKE_FOCUS() {
                protocols[protocols_len] = atom;
                protocols_len += 1;
            }
            if let Ok(atom) = self.atoms._NET_WM_PING() {
                protocols[protocols_len] = atom;
                protocols_len += 1;
            }
            match unsafe { xlib_error::sync_catch(x_display, || {
                x::XSetWMProtocols(
                    x_display, x_window, protocols.as_mut_ptr(), protocols_len as _
                )
            })} {
                Ok(success) => if success == 0 {
                    warn!("XSetWMProtocols() returned 0")
                },
                Err(e) => warn!("XSetWMProtocols() errored with {}", e),
            }
        }

        if let Ok(net_wm_pid) = self.atoms._NET_WM_PID() {
            unsafe {
                match libc::getpid() {
                    pid if pid <= 0 => warn!("getpid() returned {}; _NET_WM_PID won't be set!", pid),
                    pid => match prop::set(x_display, x_window, net_wm_pid, PropType::Cardinal, PropMode::Replace, &[pid as c_ulong]) {
                        Err(e) => warn!("Failed to set _NET_WM_PID: {}", e),
                        _ => (),
                    },
                };
            }
        }

        // Getting an X Input Context for this window
        let xic = if let Some(xim) = self.xim {
            match unsafe { xlib_error::sync_catch(x_display, || {
                x::XCreateIC(xim, 
                    x::XNClientWindow_0.as_ptr(), x_window as c_ulong,
                    x::XNFocusWindow_0.as_ptr(), x_window as c_ulong,
                    x::XNInputStyle_0.as_ptr(), (x::XIMPreeditNothing | x::XIMStatusNothing) as c_ulong,
                    ptr::null_mut() as *mut c_void,
                )
            })} {
                Err(e) => {
                    warn!("XCreateIC() reported an error: {}", e);
                    None
                },
                Ok(xic) => {
                    if xic.is_null() {
                        warn!("XCreateIC() returned NULL for X Window {}.", x_window);
                        None
                    } else {
                        trace!("Created XIC {:?} for X Window {}", xic, x_window);
                        Some(xic)
                    }
                },
            }
        } else {
            warn!("X Window {} won't have an XIC because the Context has no XIM.", x_window);
            None
        };

        let context = Rc::clone(&self.0);
        let is_cursor_visible = Cell::new(true);
        let user_cursor = RefCell::new(None);
        let window = X11Window(Rc::new(X11SharedWindow { 
            context, x_window, colormap, xic, is_cursor_visible, user_cursor
        }));
        match self.weak_windows.borrow_mut().insert(x_window, Rc::downgrade(&window.0)) {
            Some(_) => warn!("Newly created X Window {} was somewhat already present in the context's list", x_window),
            None => trace!("Inserted X Window {} into the context's list", x_window),
        }

        // Even though XCreateWindow takes x, y, w and h, window managers often ignore it
        // and place the window wherever they want.
        // We have to enforce this by setting what's called size hints for our window.
        window.x_set_wm_normal_hints(x::XSizeHints {
            flags: x::PPosition | x::PSize | x::PBaseSize,
            x, y, 
            width: w as _, 
            height: h as _,
            base_width: w as _, 
            base_height: h as _,
            // All of the below fields are ignored because of `flags` above
            min_width:  0, 
            min_height: 0,
            max_width:  0,
            max_height: 0,
            width_inc: 1,
            height_inc: 1,
            min_aspect: x::AspectRatio { x: 0, y: 0 },
            max_aspect: x::AspectRatio { x: 0, y: 0 },
            win_gravity: 0,
        });

        window.x_set_wm_hints(x::XWMHints {
            flags: x::InputHint,
            input: x::True,
            // initial_state: x::NormalState,
            .. unsafe { mem::zeroed() }
        });

        {
            let exe = env::current_exe();
            let class_name = match exe {
                Ok(ref exe) => exe.file_stem().unwrap().to_string_lossy(),
                Err(_) => env::args().nth(0).unwrap().into(),
            };
            trace!("Using \"{}\" for X Window {}'s `XClassHint` `res_name` and `res_class` strings.", class_name, x_window);
            let class_name = CString::new(class_name.into_owned()).unwrap();

            window.x_set_class_hint(x::XClassHint {
                res_name: class_name.as_bytes_with_nul().as_ptr() as *const _ as *mut _,
                res_class: class_name.as_bytes_with_nul().as_ptr() as *const _ as *mut _,
            });
        }

        unsafe {
            let argv_owned: Vec<_> = env::args().map(|s| CString::new(s).unwrap()).collect();
            let argv: Vec<_> = argv_owned.iter().map(|cs| cs.as_bytes_with_nul().as_ptr()).collect();
            window.x_set_command(argv.len() as _, argv.as_ptr() as _);
        }

        if let Err(e) = window.set_net_wm_window_type(&[NetWMWindowType::Normal]) {
            warn!("Could not set the X Window {}'s `_NET_WM_WINDOW_TYPE` to `_NET_WM_WINDOW_TYPE_NORMAL`: {}", x_window, e);
        }

        if let Err(e) = self.xi_select_all_non_raw_events_all_devices(x_window) {
            warn!("Could not select all XI non-raw events for XIAllDevices for X Window {}: {}", x_window, e);
        }

        self.x_sync();

        Ok(window)
    }

    pub fn window_from_handle(&self, x_window: x::Window, params: Option<&X11WindowFromHandleParams>) -> Result<X11Window> {
        if let Some(weak) = self.weak_windows.borrow().get(&x_window) {
            if let Some(strong) = weak.upgrade() {
                return Ok(X11Window(strong));
            } else {
                warn!("X Window {} was destroyed but not removed from the context's list", x_window);
            }
        }
        let x_display = self.x_display;
        let wa = unsafe {
            let mut wa = mem::zeroed();
            let status = xlib_error::sync_catch(x_display, || {
                x::XGetWindowAttributes(x_display, x_window, &mut wa)
            })?;
            if status != x::Success as _ {
                return failed(format!("XGetWindowAttributes() returned {}", status));
            }
            wa
        };
        let colormap = wa.colormap;
        let xic = None;
        let user_cursor = RefCell::new(None);
        let is_cursor_visible = Cell::new(true);

        if let Err(e) = self.xi_select_all_non_raw_events_all_devices(x_window) {
            warn!("Could not select all XI non-raw events for XIAllDevices for X Window {}: {}", x_window, e);
        }

        warn!("Window created from X Window `{}` will NOT have an associated XIC. Also, its Colormap will be freed along with it, and the cursor is assumed to be visible.", x_window);
        self.x_sync();
        let context = Rc::clone(&self.0);
        let window = X11Window(Rc::new(X11SharedWindow {
            context, x_window, colormap, xic, is_cursor_visible, user_cursor,
        }));
        self.weak_windows.borrow_mut().insert(x_window, Rc::downgrade(&window.0));
        trace!("Inserted foreign X Window {} into the context's list", x_window);
        Ok(window)
    }
}

impl X11SharedWindow {
    fn set_prop<T: PropElement>(&self, prop: x::Atom, prop_type: PropType, mode: PropMode, data: &[T]) -> Result<()> {
        prop::set(self.context.x_display, self.x_window, prop, prop_type, mode, data)
    }
    fn prop<T: PropElement>(&self, prop: x::Atom, req_type: PropType, long_range: Range<usize>) -> Result<PropData<T>> {
        prop::get(self.context.x_display, self.x_window, prop, req_type, long_range)
    }
    fn delete_prop(&self, prop: x::Atom) -> Result<()> {
        unsafe {
            xlib_error::sync_catch(self.context.x_display, || {
                x::XDeleteProperty(self.context.x_display, self.x_window, prop);
            })
        }
    }

    // We must use the `XAlloc..()` functions because the structs might be extended in the
    // future and only the XAlloc* functions know how big they are.
    // Silly if you ask me.
    fn x_set_wm_hints(&self, wm_hints: x::XWMHints) {
        unsafe {
            let mem = x::XAllocWMHints();
            assert_ne!(mem, ptr::null_mut());
            *mem = wm_hints;
            let status = xlib_error::sync_catch(self.context.x_display, || {
                x::XSetWMHints(self.context.x_display, self.x_window, mem)
            });
            x::XFree(mem as _);
            if let Err(e) = status {
                error!("XSetWMHints generated {}", e);
            }
        }
    }
    fn x_set_wm_normal_hints(&self, normal_hints: x::XSizeHints) {
        unsafe {
            let mem = x::XAllocSizeHints();
            assert_ne!(mem, ptr::null_mut());
            *mem = normal_hints;
            let status = xlib_error::sync_catch(self.context.x_display, || {
                x::XSetWMNormalHints(self.context.x_display, self.x_window, mem)
            });
            x::XFree(mem as _);
            if let Err(e) = status {
                error!("XSetWMNormalHints generated {}", e);
            }
        }
    }
    fn x_set_class_hint(&self, class_hint: x::XClassHint) {
        unsafe {
            let mem = x::XAllocClassHint();
            assert_ne!(mem, ptr::null_mut());
            *mem = class_hint;
            let status = xlib_error::sync_catch(self.context.x_display, || {
                x::XSetClassHint(self.context.x_display, self.x_window, mem)
            });
            x::XFree(mem as _);
            if let Err(e) = status {
                error!("XSetClassHint generated {}", e);
            }
        }
    }
    unsafe fn x_set_command(&self, argc: c_int, argv: *const *const c_char) {
        x::XSetCommand(self.context.x_display, self.x_window, argv as _, argc);
    }

    pub(crate) fn set_net_wm_user_time(&self, time: x::Time) -> Result<()> {
        self.set_prop(self.context.atoms._NET_WM_USER_TIME()?, PropType::Cardinal, PropMode::Replace, &[time])
    }

    fn set_net_wm_window_type(&self, t: &[NetWMWindowType]) -> Result<()> {
        let atoms = &self.context.atoms;
        let value: Vec<_> = t.iter().map(|t| match *t {
            NetWMWindowType::Desktop       => atoms._NET_WM_WINDOW_TYPE_DESKTOP(),
            NetWMWindowType::Dock          => atoms._NET_WM_WINDOW_TYPE_DOCK(),
            NetWMWindowType::Toolbar       => atoms._NET_WM_WINDOW_TYPE_TOOLBAR(),
            NetWMWindowType::Menu          => atoms._NET_WM_WINDOW_TYPE_MENU(),
            NetWMWindowType::Utility       => atoms._NET_WM_WINDOW_TYPE_UTILITY(),
            NetWMWindowType::Splash        => atoms._NET_WM_WINDOW_TYPE_SPLASH(),
            NetWMWindowType::Dialog        => atoms._NET_WM_WINDOW_TYPE_DIALOG(),
            NetWMWindowType::DropdownMenu  => atoms._NET_WM_WINDOW_TYPE_DROPDOWN_MENU(),
            NetWMWindowType::PopupMenu     => atoms._NET_WM_WINDOW_TYPE_POPUP_MENU(),
            NetWMWindowType::Tooltip       => atoms._NET_WM_WINDOW_TYPE_TOOLTIP(),
            NetWMWindowType::Notification  => atoms._NET_WM_WINDOW_TYPE_NOTIFICATION(),
            NetWMWindowType::Combo         => atoms._NET_WM_WINDOW_TYPE_COMBO(),
            NetWMWindowType::DND           => atoms._NET_WM_WINDOW_TYPE_DND(),
            NetWMWindowType::Normal        => atoms._NET_WM_WINDOW_TYPE_NORMAL(),
        }).filter_map(|s| s.ok()).collect();
        self.set_prop(atoms._NET_WM_WINDOW_TYPE()?, PropType::Atom, PropMode::Replace, &value)
    }
    fn set_net_wm_state(&self, action: NetWMStateAction, prop1: x::Atom, prop2: x::Atom) -> Result<()> {
        // https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html#sourceindication
        let source_indication = 1;
        self.send_client_message_to_root_window_long(
            self.context.atoms._NET_WM_STATE()?, 
            [action as _, prop1 as _, prop2 as _, source_indication]
        )
    }
    fn send_client_message_to_root_window_long(&self, message_type: x::Atom, data: [c_long; 4]) -> Result<()> {
        let x_display = self.context.x_display;

        let mut e = x::XClientMessageEvent {
            type_: x::ClientMessage,
            serial: 0,
            send_event: x::True,
            display: x_display,
            window: self.x_window,
            message_type,
            format: 32,
            data: Default::default(),
        };
        unsafe {
            e.data.set_long(0, data[0]);
            e.data.set_long(1, data[1]);
            e.data.set_long(2, data[2]);
            e.data.set_long(3, data[3]);

            let root = self.x_root_window()?;
            let event_mask = x::SubstructureNotifyMask | x::SubstructureRedirectMask;
            let status = xlib_error::sync_catch(x_display, || x::XSendEvent(
                x_display, root, x::False,
                event_mask, &mut e as *mut _ as *mut x::XEvent
            ))?;
            if status == 0 {
                return failed("XSendEvent returned 0");
            }
        }
        Ok(())
    }

    fn set_net_wm_state_fullscreen(&self, action: NetWMStateAction) -> Result<()> {
        self.set_net_wm_state(action, self.context.atoms._NET_WM_STATE_FULLSCREEN()?, 0)
    }
    fn set_bypass_compositor(&self, value: BypassCompositor) -> Result<()> {
        self.set_prop(self.context.atoms._NET_WM_BYPASS_COMPOSITOR()?, PropType::Cardinal, PropMode::Replace, &[value as c_long])
    }
    fn wm_state_property(&self) -> Result<[c_ulong; 2]> {
        let PropData {
            data, bytes_remaining_to_be_read: _,
        } = self.prop(self.context.atoms.WM_STATE()?, PropType::Any, 0..2)?;
        Ok([data[0], data[1]])
    }
    fn net_wm_allowed_actions(&self) -> Result<Vec<c_ulong>> {
        self.prop(self.context.atoms._NET_WM_ALLOWED_ACTIONS()?, PropType::Any, 0..64).map(|pd| pd.data)
    }
    fn net_wm_state(&self) -> Result<Vec<c_ulong>> {
        self.prop(self.context.atoms._NET_WM_STATE()?, PropType::Any, 0..64).map(|pd| pd.data)
    }
    fn set_net_wm_allowed_action(&self, action: x::Atom, allow: bool) -> Result<()> {
        let mut allowed_actions = self.net_wm_allowed_actions()?;
        let i = allowed_actions.iter().cloned()
            .enumerate()
            .filter(|&(_, a)| a == action)
            .next();
        let was_allowed = i.is_some();
        if was_allowed == allow {
            return Ok(());
        }
        let prop = self.context.atoms._NET_WM_ALLOWED_ACTIONS()?;
        if allow {
            self.set_prop(prop, PropType::Atom, PropMode::Append, &[action])
        } else {
            allowed_actions.swap_remove(i.unwrap().0);
            self.set_prop(prop, PropType::Atom, PropMode::Replace, &allowed_actions)
        }
    }
    fn motif_wm_hints(&self) -> Result<motif_wm::Hints> {
        let PropData {
            data: d, bytes_remaining_to_be_read: _,
        } = self.prop(self.context.atoms._MOTIF_WM_HINTS()?, PropType::Any, 0..5)?;
        Ok([d[0], d[1], d[2], d[3], d[4]].into())
    }
    fn set_motif_wm_hints(&self, hints: motif_wm::Hints) -> Result<()> {
        let hints = hints.into_array();
        self.set_prop(self.context.atoms._MOTIF_WM_HINTS()?, PropType::Cardinal, PropMode::Replace, &hints)
    }

    fn x_geometry_and_root(&self) -> Result<(Rect<i32, u32>, x::Window)> {
        unsafe {
            let x_display = self.context.x_display;
            let mut out_root: x::Window = 0;
            let (mut x, mut y): (c_int, c_int) = (0, 0);
            let (mut w, mut h): (c_uint, c_uint) = (0, 0);
            let mut border: c_uint = 0;
            let mut depth: c_uint = 0;
            self.context.x_sync();
            // Inoring BadDrawable, BadWindow
            let status = x::XGetGeometry(
                x_display, self.x_window, &mut out_root,
                &mut x, &mut y, &mut w, &mut h, &mut border, &mut depth
            );
            if status == 0 {
                return failed(format!("XGetGeometry() returned {}", status))
            }
            let geom = Rect {
                x: x as _,
                y: y as _,
                w: w as _,
                h: h as _,
            };
            Ok((geom, out_root))
        }
    }
    fn x_geometry(&self) -> Result<Rect<i32, u32>> {
        self.x_geometry_and_root().map(|r| r.0)
    }
    fn x_root_window(&self) -> Result<x::Window> {
        self.x_geometry_and_root().map(|r| r.1)
    }
}

impl X11SharedWindow {

    pub fn handle(&self) -> WindowHandle {
        WindowHandle(self.x_window)
    }

    pub fn set_type_hint(&self, type_hint: &WindowTypeHint) -> Result<()> {
        let is_visible = self.is_visible();
        if is_visible.is_ok() && is_visible.unwrap() {
            warn!("You're supposed to call `set_type_hint()` *before* showing the window for the first time (X Window {}). The result is implementation-dependant.", self.x_window);
        }
        self.set_net_wm_window_type(type_hint.net_wm)
    }

    pub fn title(&self) -> Result<String> {
        failed("This is not implemented yet")
    }
    pub fn set_title(&self, title: &str) -> Result<()> {
        unsafe {
            let x_display = self.context.x_display;
            let c_title = match CString::new(title) {
                Ok(s) => s,
                Err(e) => return failed(format!("Could not convert title to UTF-8: {}", e)),
            };
            let c_title_ptr = c_title.as_bytes_with_nul().as_ptr() as *mut c_char;
            let title_ptr = &mut [c_title_ptr];
            let mut prop: x::XTextProperty = mem::uninitialized();
            let status = x::Xutf8TextListToTextProperty(
                x_display, title_ptr.as_mut_ptr(), title_ptr.len() as _, x::XUTF8StringStyle, &mut prop
            );
            match status {
                s if s == x::Success as c_int => {
                    // Ignoring BadAlloc, BadWindow
                    x::XSetWMName(x_display, self.x_window, &mut prop);
                    x::XSetWMIconName(x_display, self.x_window, &mut prop);
                    if let Ok(net_wm_name) = self.context.atoms._NET_WM_NAME() {
                        // BadAlloc, BadAtom, BadValue, BadWindow
                        x::XSetTextProperty(x_display, self.x_window, &mut prop, net_wm_name);
                    }
                    x::XFree(prop.value as _);
                    self.context.x_flush();
                    Ok(())
                },
                missing_bits::xutil::XNoMemory => failed("Xutf8TextListToTextProperty() returned XNoMemory"),
                missing_bits::xutil::XLocaleNotSupported => failed("Xutf8TextListToTextProperty() returned XLocaleNotSupported"),
                _ => failed_unexplained(),
            }
        }
    }

    pub fn reset_icon(&self) -> Result<()> {
        self.delete_prop(self.context.atoms._NET_WM_ICON()?)
    }

    pub fn icon(&self) -> Result<(Extent2<u32>, Vec<Rgba<u8>>)> {
        failed("This is not implemnted yet")
    }
    pub fn set_icon(&self, size: Extent2<u32>, data: &[Rgba<u8>]) -> Result<()> {
        let (w, h) = size.into_tuple();
        let mut propdata = Vec::<c_ulong>::with_capacity((2 + w * h) as _);
        propdata.push(w as _);
        propdata.push(h as _);
        for y in 0..h {
            for x in 0..w {
                let p: Rgba<u8> = data[(y*w + x) as usize];
                let argb = 
                      (p.a as u32) << 24
                    | (p.r as u32) << 16
                    | (p.g as u32) << 8
                    | (p.b as u32);
                propdata.push(argb as _);
            }
        }
        self.set_prop(self.context.atoms._NET_WM_ICON()?, PropType::Cardinal, PropMode::Replace, &propdata)
    }

    pub fn set_style_hint(&self, style_hint: &WindowStyleHint) -> Result<()> {
        use self::motif_wm::{flags, decorations, functions};

        let &WindowStyleHint {
            title_bar_features, borders
        } = style_hint;

        // NOTE: unwrap_or_default() because window might not have that property at first,
        // even though the WM supports _MOTIF_WM_HINTS.
        let mut hints = self.motif_wm_hints().unwrap_or_default();

        if let Some(window::Borders { thickness: _, color: _ }) = borders {
            hints.flags |= flags::DECORATIONS;
            hints.decorations |= decorations::BORDER;
        } else {
            hints.decorations &= !decorations::BORDER;
        }

        if let Some(window::TitleBarFeatures {
            minimize, maximize, close
        }) = title_bar_features {
            hints.flags |= flags::DECORATIONS;
            hints.decorations |= decorations::TITLE | decorations::MENU;
            if minimize {
                hints.decorations |= decorations::MINIMIZE;
            } else {
                hints.decorations &= !decorations::MINIMIZE;
            }
            if maximize {
                hints.decorations |= decorations::MAXIMIZE;
            } else {
                hints.decorations &= !decorations::MAXIMIZE;
            }
            if close {
                hints.flags |= flags::FUNCTIONS;
                hints.functions   |= functions::CLOSE;
            } else {
                hints.functions   &= !functions::CLOSE;
            }
        } else {
            hints.decorations &= !(decorations::TITLE | decorations::MENU);
        }

        self.set_motif_wm_hints(hints)
    }
    
    pub fn is_minimized(&self) -> Result<bool> {
        Ok(self.wm_state_property()?[0] == missing_bits::wm_state::IconicState as _)
    }
    pub fn is_visible(&self) -> Result<bool> {
        Ok(self.wm_state_property()?[0] != missing_bits::wm_state::WithdrawnState as _)
    }
    pub fn toggle_minimize(&self) -> Result<()> {
        if self.is_minimized()? {
            self.unminimize()
        } else {
            self.minimize()
        }
    }
    pub fn toggle_visibility(&self) -> Result<()> {
        if self.is_visible()? {
            self.hide()
        } else {
            self.show()
        }
    }

    pub fn is_maximized(&self) -> Result<bool> {
        let state = self.net_wm_state()?;
        Ok(state.contains(&self.context.atoms._NET_WM_STATE_MAXIMIZED_VERT()?)
        && state.contains(&self.context.atoms._NET_WM_STATE_MAXIMIZED_HORZ()?))
    }
    pub fn is_width_maximized(&self) -> Result<bool> {
        Ok(self.net_wm_state()?.contains(&self.context.atoms._NET_WM_STATE_MAXIMIZED_HORZ()?))
    }
    pub fn is_height_maximized(&self) -> Result<bool> {
        Ok(self.net_wm_state()?.contains(&self.context.atoms._NET_WM_STATE_MAXIMIZED_VERT()?))
    }
    pub fn is_fullscreen(&self) -> Result<bool> {
        Ok(self.net_wm_state()?.contains(&self.context.atoms._NET_WM_STATE_FULLSCREEN()?))
    }

    pub fn set_opacity(&self, alpha: f64) -> Result<()> {
        self.set_prop(
            self.context.atoms._NET_WM_WINDOW_OPACITY()?,
            PropType::Cardinal, PropMode::Replace,
            &[(0xFFFFFFFF_u32 as f64 * alpha.clamped01()) as c_ulong]
        )
    }

    pub fn show(&self) -> Result<()> {
        unsafe {
            x::XMapWindow(self.context.x_display, self.x_window);
        }
        // Sync, otherwise it would be possible
        // to swap buffers before the window is shown, which would
        // have no effect and be surprising.
        self.context.x_sync();
        Ok(())
    }
    pub fn hide(&self) -> Result<()> {
        unsafe {
            x::XUnmapWindow(self.context.x_display, self.x_window);
        }
        // Sync so as not to be surprising
        self.context.x_sync();
        Ok(())
    }
    pub fn raise(&self) -> Result<()> {
        unsafe {
            x::XRaiseWindow(self.context.x_display, self.x_window);
        }
        // Sync so as not to be surprising
        self.context.x_sync();
        Ok(())
    }

    pub fn maximize_height(&self) -> Result<()> {
        self.set_net_wm_state(NetWMStateAction::Add, self.context.atoms._NET_WM_STATE_MAXIMIZED_VERT()?, 0)
    }
    pub fn unmaximize_height(&self) -> Result<()> {
        self.set_net_wm_state(NetWMStateAction::Remove, self.context.atoms._NET_WM_STATE_MAXIMIZED_VERT()?, 0)
    }
    pub fn toggle_maximize_height(&self) -> Result<()> {
        self.set_net_wm_state(NetWMStateAction::Toggle, self.context.atoms._NET_WM_STATE_MAXIMIZED_VERT()?, 0)
    }
    pub fn maximize_width(&self) -> Result<()> {
        self.set_net_wm_state(NetWMStateAction::Add, self.context.atoms._NET_WM_STATE_MAXIMIZED_HORZ()?, 0)
    }
    pub fn unmaximize_width(&self) -> Result<()> {
        self.set_net_wm_state(NetWMStateAction::Remove, self.context.atoms._NET_WM_STATE_MAXIMIZED_HORZ()?, 0)
    }
    pub fn toggle_maximize_width(&self) -> Result<()> {
        self.set_net_wm_state(NetWMStateAction::Toggle, self.context.atoms._NET_WM_STATE_MAXIMIZED_HORZ()?, 0)
    }

    pub fn maximize(&self) -> Result<()> {
        self.set_net_wm_state(NetWMStateAction::Add,
            self.context.atoms._NET_WM_STATE_MAXIMIZED_VERT()?,
            self.context.atoms._NET_WM_STATE_MAXIMIZED_HORZ()?
        )
    }
    pub fn unmaximize(&self) -> Result<()> {
        self.set_net_wm_state(NetWMStateAction::Remove,
            self.context.atoms._NET_WM_STATE_MAXIMIZED_VERT()?,
            self.context.atoms._NET_WM_STATE_MAXIMIZED_HORZ()?
        )
    }
    pub fn toggle_maximize(&self) -> Result<()> {
        self.set_net_wm_state(NetWMStateAction::Toggle,
            self.context.atoms._NET_WM_STATE_MAXIMIZED_VERT()?,
            self.context.atoms._NET_WM_STATE_MAXIMIZED_HORZ()?
        )
    }
    pub fn minimize(&self) -> Result<()> {
        let status = unsafe { xlib_error::sync_catch(self.context.x_display, || {
            x::XIconifyWindow(
                self.context.x_display, self.x_window,
                self.context.x_default_screen_num()
            )
        })};
        match status {
            Err(e) => Err(e),
            Ok(0) => failed(format!("XIconifyWindow() returned 0")),
            Ok(_) => Ok(()),
        }
    }
    pub fn unminimize(&self) -> Result<()> {
        self.show()
    }
    pub fn toggle_fullscreen(&self) -> Result<()> {
        let _ = self.set_bypass_compositor(BypassCompositor::NoPreference);
        self.set_net_wm_state_fullscreen(NetWMStateAction::Toggle)
    }
    pub fn enter_fullscreen(&self) -> Result<()> {
        // NOTE: Bypassing the compositor doesn't appear to discard the window's opacity value,
        // but other window managers might not be as clever as the one I've tested this on (xfwm4).
        let _ = self.set_bypass_compositor(BypassCompositor::Yes);
        self.set_net_wm_state_fullscreen(NetWMStateAction::Add)
    }
    pub fn leave_fullscreen(&self) -> Result<()> {
        let _ = self.set_bypass_compositor(BypassCompositor::NoPreference);
        self.set_net_wm_state_fullscreen(NetWMStateAction::Remove)
    }
    pub fn demand_attention(&self) -> Result<()> {
        // NOTE: This is automatically reset by the window manager when it decides that
        // the window got the requested attention.
        self.set_net_wm_state(
            NetWMStateAction::Add,
            self.context.atoms._NET_WM_STATE_DEMANDS_ATTENTION()?, 0
        )
    }
    pub fn demand_urgent_attention(&self) -> Result<()> {
        let _ = self.demand_attention();
        self.x_set_wm_hints(x::XWMHints {
            flags: x::XUrgencyHint,
            .. unsafe { mem::zeroed() }
        });
        Ok(())
    }

    pub fn is_movable(&self) -> Result<bool> {
        Ok(self.net_wm_allowed_actions()?.contains(&self.context.atoms._NET_WM_ACTION_MOVE()?))
    }
    pub fn is_resizable(&self) -> Result<bool> {
        Ok(self.net_wm_allowed_actions()?.contains(&self.context.atoms._NET_WM_ACTION_RESIZE()?))
    }
    pub fn set_movable(&self, movable: bool) -> Result<()> {
        // NOTE: unwrap_or_default() because window might not have that property at first,
        // even though the WM supports _MOTIF_WM_HINTS.
        let mut hints = self.motif_wm_hints().unwrap_or_default();
        if movable {
            hints.flags |= motif_wm::flags::FUNCTIONS;
            hints.functions |= motif_wm::functions::MOVE;
        } else {
            hints.functions &= !motif_wm::functions::MOVE;
        }
        let _ = self.set_motif_wm_hints(hints);
        self.set_net_wm_allowed_action(self.context.atoms._NET_WM_ACTION_MOVE()?, movable)
    }
    pub fn set_resizable(&self, resizable: bool) -> Result<()> {
        // NOTE: unwrap_or_default() because window might not have that property at first,
        // even though the WM supports _MOTIF_WM_HINTS.
        let mut hints = self.motif_wm_hints().unwrap_or_default();
        if resizable {
            hints.flags |= motif_wm::flags::FUNCTIONS | motif_wm::flags::DECORATIONS;
            hints.functions |= motif_wm::functions::RESIZE;
            hints.decorations |= motif_wm::decorations::RESIZE;
        } else {
            hints.functions &= !motif_wm::functions::RESIZE;
            hints.decorations &= !motif_wm::decorations::RESIZE;
        }
        let _ = self.set_motif_wm_hints(hints);
        self.set_net_wm_allowed_action(self.context.atoms._NET_WM_ACTION_RESIZE()?, resizable)
    }
    pub fn set_min_size(&self, size: Extent2<u32>) -> Result<()> {
        self.x_set_wm_normal_hints(x::XSizeHints {
            flags: x::PMinSize,
            min_width: size.w as _,
            min_height: size.h as _,
            .. unsafe { mem::zeroed() }
        });
        Ok(())
    }
    pub fn set_max_size(&self, size: Extent2<u32>) -> Result<()> {
        self.x_set_wm_normal_hints(x::XSizeHints {
            flags: x::PMaxSize,
            max_width: size.w as _,
            max_height: size.h as _,
            .. unsafe { mem::zeroed() }
        });
        Ok(())
    }
    pub fn position_and_size(&self) -> Result<Rect<i32, u32>> {
        self.x_geometry()
    }
    pub fn position(&self) -> Result<Vec2<i32>> {
        // WISH: Maybe use XTranslateCoordinates() instead ?
        let Rect { x, y, .. } = self.x_geometry()?;
        Ok(Vec2 { x, y })
    }
    pub fn size(&self) -> Result<Extent2<u32>> {
        let Rect { w, h, .. } = self.x_geometry()?;
        Ok(Extent2 { w, h })
    }
    pub fn canvas_size(&self) -> Result<Extent2<u32>> {
        self.size()
    }

    pub fn set_position(&self, pos: Vec2<i32>) -> Result<()> {
        unsafe {
            x::XMoveWindow(self.context.x_display, self.x_window, pos.x, pos.y);
        }
        self.context.x_sync();
        Ok(())
    }
    pub fn set_size(&self, size: Extent2<u32>) -> Result<()> {
        unsafe {
            x::XResizeWindow(self.context.x_display, self.x_window, size.w, size.h);
        }
        self.context.x_sync();
        Ok(())
    }
    pub fn set_position_and_size(&self, r: Rect<i32, u32>) -> Result<()> {
        unsafe {
            x::XMoveResizeWindow(self.context.x_display, self.x_window, r.x, r.y, r.w, r.h);
        }
        self.context.x_sync();
        Ok(())
    }
    pub fn set_desktop(&self, i: usize) -> Result<()> {
        if !self.is_visible().unwrap_or(true) {
            warn!("Setting the desktop via _NET_WM_DESKTOP might not work on withdrawn windows");
        }
        // https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html#sourceindication
        let source_indication = 1; 
        self.send_client_message_to_root_window_long(
            self.context.atoms._NET_WM_DESKTOP()?, [i as _, source_indication, 0, 0]
        )
    }
    pub fn recenter_in_desktop(&self) -> Result<()> {
        unimplemented!{}
    }
    pub fn recenter_in_work_area(&self) -> Result<()> {
        unimplemented!{}
    }


    pub fn set_mouse_position(&self, pos: Vec2<i32>) -> Result<()> {
        unsafe {
            x::XWarpPointer(
                self.context.x_display, 0, self.x_window,
                0, 0, 0, 0, pos.x as _, pos.y as _
            );
        }
        self.context.x_sync();
        Ok(())
    }
    pub fn mouse_position(&self) -> Result<Vec2<i32>> {
        let mut root: x::Window = 0;
        let mut child: x::Window = 0;
        let mut root_x: c_int = 0;
        let mut root_y: c_int = 0;
        let mut x: c_int = 0;
        let mut y: c_int = 0;
        let mut mask: c_uint = 0;
        self.context.x_sync();
        let _is_on_same_screen = unsafe {
            x::XQueryPointer(
                self.context.x_display, self.x_window,
                &mut root, &mut child, &mut root_x, &mut root_y,
                &mut x, &mut y, &mut mask
            )
            // TODO: So we also get the root window, absolute position,
            // and button mask for free. We should provide these as well!
            // Like a mouse_state() method or something
        };
        Ok(Vec2::new(x as _, y as _))
    }

    pub fn trap_mouse(&self) -> Result<()> {
        let mask = x::ButtonPressMask | x::ButtonReleaseMask | x::PointerMotionMask | x::FocusChangeMask;
        let confine_to = self.x_window;
        let cursor = 0;
        let status = unsafe {
            x::XGrabPointer(
                self.context.x_display, self.x_window, x::False,
                mask as _, x::GrabModeAsync, x::GrabModeAsync,
                confine_to, cursor, x::CurrentTime
            )
        };
        let reason = match status {
            x::GrabSuccess => return Ok(()),
            x::GrabNotViewable => "GrabNotViewable",
            x::AlreadyGrabbed  => "AlreadyGrabbed",
            x::GrabFrozen      => "GrabFrozen",
            x::GrabInvalidTime => "GrabInvalidTime",
            _ => "garbage",
        };
        failed(format!("XGrabPointer() returned {}", reason))
    }
    pub fn mouse_state(&self, mouse: DeviceID) -> device::Result<WindowMouseState> {
        unimplemented!{}
    }
    pub fn tablet_state(&self, tablet: DeviceID) -> device::Result<WindowTabletState> {
        unimplemented!{}
    }
}

