use std::ptr;
use std::mem;
use std::ffi::CStr;
use std::rc::{Rc, Weak};
use std::cell::{RefCell, Cell};
use std::slice;
use std::ops::{Deref, Range};
use std::os::raw::{c_int, c_long, c_ulong, c_uchar, c_char};
use std::collections::{HashMap, VecDeque};

use context::Context;
use desktop::Desktop;
use error::{Result, failed};
use event::Event;
use os::OsContext;
use {Rect, Vec2};

use super::x11::xlib as x;
use super::x11::xinput2 as xi2;
use super::atoms;
use super::prop::{self, PropType, PropElement, PropData};
use super::xrender;
use super::device::{XI2DeviceCache};
use super::xi;
use super::glx;
use super::X11SharedWindow;


/// On X11-based targets, a `Context` **owns** an Xlib `Display` pointer.
impl Context {
    /// X11-only specialization of `new()` where you can specify
    /// the name given to `XOpenDisplay()`.
    pub fn with_x11_display_name(name: Option<&CStr>) -> Result<Self> {
        X11Context::with_x11_display_name(name).map(OsContext::from).map(Context)
    }
    /// X11-only specialization of `new()` where you **transfer ownership**
    /// of an existing, valid Xlib `Display` pointer.
    ///
    /// This function is unsafe because there's no guarantee that the pointer is valid.
    pub unsafe fn from_xlib_display(dpy: *mut x::Display) -> Result<Self> {
        X11Context::from_xlib_display(dpy).map(OsContext::from).map(Context)
    }
    /// (X11-only) Gets the `Display` pointer associated with this `Context`.
    ///
    /// Be careful: It is closed when the `Context` is dropped.  
    /// Also, it is not locked via `XLockDisplay()`. It's up to you to call it
    /// if necessary and call `XUnlockDisplay()` as appropriate.
    pub fn xlib_display(&self) -> *mut x::Display {
        self.0.x11.x11_owned_display.0
    }
    /// (X11-only) Calls `XSynchronize` to enable or disable synchronous behaviour.
    ///
    /// This should be avoided but is useful for debugging.  
    /// See also
    /// [this page](https://tronche.com/gui/x/xlib/event-handling/protocol-errors/synchronization.html).
    pub fn xlib_xsynchronize(&self, enable: bool) {
        unsafe {
            x::XSynchronize(*self.0.x11.lock_x_display(), enable as _);
        }
    }
}

#[derive(Debug)]
pub struct X11Context(pub Rc<X11SharedContext>);

/// An "owned" Xlib `Display` pointer, which is closed when dropped.
#[derive(Debug)]
pub struct X11OwnedDisplay(*mut x::Display);

#[derive(Debug)]
pub struct X11LockedDisplay<'a>(*mut x::Display, ::std::marker::PhantomData<&'a ()>);

impl X11OwnedDisplay {
    // `XLockDisplay()` is fine to call anywhere as long as there's a matching
    // `XUnlockDisplay()` in the end.
    //
    // Calls to `XLockDisplay()` can safely be nested (as if there was an internal refcount).
    //
    // In addition, we pay no cost for `XLockDisplay()` and `XUnlockDisplay()` if `XInitThreads()`
    // was not called in the first place.
    fn lock<'a>(&'a self) -> X11LockedDisplay<'a> {
        unsafe {
            x::XLockDisplay(self.0);
        }
        X11LockedDisplay(self.0, ::std::marker::PhantomData)
    }
}
impl<'a> Drop for X11LockedDisplay<'a> {
    fn drop(&mut self) {
        unsafe {
            x::XUnlockDisplay(self.0);
        }
    }
}
impl Drop for X11OwnedDisplay {
    fn drop(&mut self) {
        unsafe {
            close_x_display(self.0)
        }
    }
}

impl<'a> Deref for X11LockedDisplay<'a> {
    type Target = *mut x::Display;
    fn deref(&self) -> &*mut x::Display {
        &self.0
    }
}

impl X11SharedContext {
    pub fn lock_x_display<'a>(&'a self) -> X11LockedDisplay<'a> {
        self.x11_owned_display.lock()
    }
}

#[derive(Debug)]
pub struct X11SharedContext {
    x11_owned_display: X11OwnedDisplay,
    pub xim: Option<x::XIM>,
    pub atoms: atoms::PreloadedAtoms,
    pub xrender: Result<xrender::XRender>,
    pub xi: Result<xi::XI>,
    pub glx: Result<glx::Glx>,
    pub invisible_x_cursor: x::Cursor,
    pub default_x_cursor: x::Cursor,
    pub weak_windows: RefCell<HashMap<x::Window, Weak<X11SharedWindow>>>,
    pub pending_translated_events: RefCell<VecDeque<Event>>,
    // These two fields are used to detect key repeat events.
    pub previous_mouse_position: Cell<Option<Vec2<f64>>>,
    pub previous_xi_raw_key_event: Cell<(c_int, x::Time, x::KeyCode)>,
    pub xi2_devices: RefCell<HashMap<c_int, XI2DeviceCache>>,
}

impl Deref for X11Context {
    type Target = X11SharedContext;
    fn deref(&self) -> &X11SharedContext {
        &self.0
    }
}

impl Drop for X11SharedContext {
    fn drop(&mut self) {
        let &mut Self {
            x11_owned_display: _, xim, atoms: _, xrender: _, xi: _, glx: _,
            invisible_x_cursor, default_x_cursor, weak_windows: _,
            pending_translated_events: _,
            previous_mouse_position: _,
            previous_xi_raw_key_event: _,
            xi2_devices: _,
        } = self;
        let x_display = self.lock_x_display();
        unsafe {
            x::XSync(*x_display, x::False);
            x::XFreeCursor(*x_display, invisible_x_cursor);
            x::XFreeCursor(*x_display, default_x_cursor);
            if let Some(xim) = xim {
                x::XCloseIM(xim);
                trace!("Closed XIM {:?}", xim);
            }
        }
    }
}

unsafe fn close_x_display(x_display: *mut x::Display) {
    x::XSync(x_display, x::False);
    let name = {
        let p = x::XDisplayString(x_display);
        CStr::from_ptr(p).to_string_lossy().into_owned()
        // ^ into_owned() is critical here to clone the C string
        // before closing the display.
    };
    x::XCloseDisplay(x_display);
    trace!("Closed X Display `{}`", name);
}


#[derive(Debug, Default, Clone)]
pub struct ExtensionInfo {
    pub name: String,
    pub major_opcode: c_int,
    pub first_event: c_int,
    pub first_error: c_int,
}

// XXX: Assumes we'll only ever have a single display
// This is global for error handling.
pub static mut ALL_EXTENSIONS: Option<HashMap<c_int, ExtensionInfo>> = None;

unsafe fn init_all_extensions(x_display: *mut x::Display) {
    assert!(!x_display.is_null());
    let mut all_extensions = HashMap::with_capacity(32); // reasonable. Try `xdpyinfo -queryExt`.
    let mut nextensions = 0;
    let list_ptr: *mut *mut c_char = x::XListExtensions(x_display, &mut nextensions);
    if list_ptr.is_null() {
        // Paranoid, should never happen...
        ALL_EXTENSIONS = Some(HashMap::new());
        return;
    }
    assert!(nextensions >= 0);
    let list = slice::from_raw_parts(list_ptr, nextensions as usize);
    for name in list.iter().cloned() {
        if name.is_null() {
            continue; // Paranoid. Should never happen.
        }
        let mut ext = ExtensionInfo {
            name: CStr::from_ptr(name).to_string_lossy().into_owned(),
            major_opcode: 0,
            first_event: 0,
            first_error: 0,
        };
        let is_ok = x::True == x::XQueryExtension(x_display, name, &mut ext.major_opcode, &mut ext.first_event, &mut ext.first_error);
        if is_ok {
            trace!("Found X11 extension: {:?}", ext);
            all_extensions.insert(ext.major_opcode, ext);
        }
    }
    x::XFreeExtensionList(list_ptr);
    ALL_EXTENSIONS = Some(all_extensions);
}


impl X11Context {
    pub fn new() -> Result<Self> {
        Self::with_x11_display_name(None)
    }

    pub fn with_x11_display_name(x_display_name: Option<&::std::ffi::CStr>) -> Result<Self> {
        let x_display_name_ptr = match x_display_name {
            Some(s) => s.as_ptr(),
            None => ptr::null(),
        };
        let x_display = unsafe {
            x::XOpenDisplay(x_display_name_ptr)
        };
        let actual_name = unsafe {
            // NOTE: Still works if x_display is NULL. That's the point.
            CStr::from_ptr(x::XDisplayString(x_display)).to_string_lossy()
        };
        if x_display.is_null() {
            return failed(format!("Failed to open X display `{}`", actual_name));
        }
        trace!("Opened X Display `{}`", actual_name);
        Self::from_x11_owned_display(X11OwnedDisplay(x_display))
    }

    pub unsafe fn from_xlib_display(x_display: *mut x::Display) -> Result<Self> {
        assert!(!x_display.is_null());
        Self::from_x11_owned_display(X11OwnedDisplay(x_display))
    }

    pub fn from_x11_owned_display(x11_owned_display: X11OwnedDisplay) -> Result<Self> {
        let mut c = unsafe {
            let x_display = x11_owned_display.lock();
            let screen_count      = x::XScreenCount(*x_display);
            let protocol_version  = x::XProtocolVersion(*x_display);
            let protocol_revision = x::XProtocolRevision(*x_display);
            let vendor_release    = x::XVendorRelease(*x_display);
            let server_vendor     = CStr::from_ptr(x::XServerVendor(*x_display)).to_string_lossy();
            trace!("X protocol version {}, revision {}", protocol_version, protocol_revision);
            trace!("X server vendor: `{}`, release {}", server_vendor, vendor_release);
            trace!("X server screen count: {}", screen_count);

            init_all_extensions(*x_display);

            let atoms = atoms::PreloadedAtoms::load(*x_display)?; // Sneaky return, watch out for unmanaged resources created before this line!
            let invisible_x_cursor = super::cursor::create_invisible_x_cursor(*x_display);
            let default_x_cursor = super::cursor::create_default_x_cursor(*x_display);
            let xrender = super::xrender::XRender::query(*x_display);
            let xi = super::xi::XI::query(*x_display);
            let glx = super::glx::Glx::query(*x_display);

            let xim = {
                let (db, res_name, res_class) = (ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
                let xim = x::XOpenIM(*x_display, db, res_name, res_class);
                if xim.is_null() {
                    warn!("XOpenIM() returned NULL");
                    None
                } else {
                    trace!("Opened XIM {:?}", xim);
                    Some(xim)
                }
            };

            let previous_mouse_position = Cell::new(None);
            let previous_xi_raw_key_event = Cell::default();
            let pending_translated_events = RefCell::new(VecDeque::new());
            let weak_windows = RefCell::new(HashMap::new());
            let xi2_devices = RefCell::new(super::device::xi2_query_device_info(*x_display, xi2::XIAllDevices, &atoms)
                .unwrap()
                .into_iter()
                .map(|(deviceid, info)| {
                    let props = atoms.interesting_xi2_props()
                        .iter()
                        .filter_map(|k| super::device::xi2_get_device_property(*x_display, deviceid, *k).ok().map(|v| (k, v)))
                        .filter_map(|(k, v)| v.map(|v| (*k, v)))
                        .collect();
                    (deviceid, XI2DeviceCache { info, props })
                })
                .collect());

            X11SharedContext {
                xim, atoms, xrender, xi, glx, invisible_x_cursor, default_x_cursor,
                weak_windows, pending_translated_events,
                previous_mouse_position,
                previous_xi_raw_key_event,
                xi2_devices,
                x11_owned_display: mem::zeroed(), // Can't move x11_owned_display because it is borrowed
            }
        };
        mem::forget(mem::replace(&mut c.x11_owned_display, x11_owned_display));
        Ok(X11Context(Rc::new(c)))
    }
}


impl X11SharedContext {
    pub fn x_default_screen(&self) -> *mut x::Screen {
        unsafe {
            x::XDefaultScreenOfDisplay(*self.lock_x_display())
        }
    }
    pub fn x_default_screen_num(&self) -> c_int {
        unsafe {
            x::XDefaultScreen(*self.lock_x_display())
        }
    }
    pub fn x_default_root_window(&self) -> x::Window {
        unsafe {
            x::XDefaultRootWindow(*self.lock_x_display())
        }
    }
    pub fn x_default_visual(&self) -> *mut x::Visual {
        unsafe {
            x::XDefaultVisual(*self.lock_x_display(), self.x_default_screen_num())
        }
    }
    /// XFlush() flushes the output buffer.
    pub fn x_flush(&self) {
        unsafe {
            x::XFlush(*self.lock_x_display());
        }
    }
    /// XSync() is like XFlush() except that it also waits for all requests
    /// to be processed. In particular, this ensures that the error handler is
    /// called before going any further.
    pub fn x_sync(&self) {
        unsafe {
            x::XSync(*self.lock_x_display(), x::False);
        }
    }
    #[allow(dead_code)]
    fn x_sync_discarding_all_events_in_the_queue(&self) {
        unsafe {
            x::XSync(*self.lock_x_display(), x::True);
        }
    }

    fn root_prop<T: PropElement>(&self, prop: x::Atom, req_type: PropType, long_range: Range<usize>) -> Result<PropData<T>> {
        prop::get(*self.lock_x_display(), self.x_default_root_window(), prop, req_type, long_range)
    }

    fn number_of_desktops(&self) -> Result<usize> {
        self.root_prop::<c_ulong>(self.atoms._NET_NUMBER_OF_DESKTOPS()?, PropType::Any, 0..1).map(|pd| pd.data[0] as _)
    }
    pub fn current_desktop(&self) -> Result<usize> {
        self.root_prop::<c_ulong>(self.atoms._NET_CURRENT_DESKTOP()?, PropType::Any, 0..1).map(|pd| pd.data[0] as _)
    }
    pub fn desktops(&self) -> Result<Vec<Desktop>> {
        let nb_desktops = self.number_of_desktops()?;

        let PropData {
            data: work_areas, bytes_remaining_to_be_read: _,
        } = self.root_prop::<c_long>(self.atoms._NET_WORKAREA()?, PropType::Any, 0..(4*nb_desktops))?;

        let PropData {
            data: raw_utf8_names, bytes_remaining_to_be_read: _,
        } = self.root_prop::<c_uchar>(self.atoms._NET_DESKTOP_NAMES()?, PropType::Any, 0..4096)?;

        let names = {
            let mut names = Vec::with_capacity(nb_desktops);
            let mut start = 0;
            for i in 0..raw_utf8_names.len() {
                if raw_utf8_names[i] == 0 {
                    names.push(String::from_utf8(raw_utf8_names[start..i].to_vec()));
                    start = i+1;
                }
            }
            names
        };

        let mut desktops = Vec::with_capacity(nb_desktops);
        for i in 0..nb_desktops {
            desktops.push(Desktop {
                name: names.get(i).map(|name| name.as_ref().ok().map(Clone::clone)).unwrap_or(None),
                work_area: Rect {
                    x: work_areas[4*i + 0] as _,
                    y: work_areas[4*i + 1] as _,
                    w: work_areas[4*i + 2] as _,
                    h: work_areas[4*i + 3] as _,
                },
            });
        }
        Ok(desktops)
    }
    pub fn untrap_mouse(&self) -> Result<()> {
        unsafe {
            // No status or error to check for.
            x::XUngrabPointer(*self.lock_x_display(), x::CurrentTime);
        }
        Ok(())
    }
}

