use std::ptr;
use std::ffi::CStr;
use std::rc::Rc;
use std::ops::{Deref, Range};
use std::os::raw::{c_int, c_long, c_ulong, c_uchar};

use context::Context;
use desktop::Desktop;
use error::{Result, failed};
use os::OsContext;
use Rect;

use super::x11::xlib as x;
use super::atoms;
use super::prop::{self, PropType, PropElement, PropData};
use super::xrender;
use super::xi;


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
    pub fn xlib_display(&self) -> *mut x::Display {
        self.0.x11.x_display
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct X11Context(pub Rc<X11SharedContext>);

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct X11SharedContext {
    pub x_display: *mut x::Display,
    pub xim: Option<x::XIM>,
    pub atoms: atoms::PreloadedAtoms,
    pub xrender: Result<xrender::XRender>,
    pub xi: Result<xi::XI>,
    pub invisible_x_cursor: x::Cursor,
    pub default_x_cursor: x::Cursor,
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
            x_display, xim, atoms: _, xrender: _, xi: _,
            invisible_x_cursor, default_x_cursor,
        } = self;
        unsafe {
            x::XFreeCursor(x_display, invisible_x_cursor);
            x::XFreeCursor(x_display, default_x_cursor);
            if let Some(xim) = xim {
                x::XCloseIM(xim);
                trace!("Closed XIM {:?}", xim);
            }
            close_x_display(x_display);
        }
    }
}

unsafe fn close_x_display(x_display: *mut x::Display) {
    let name = {
        let p = x::XDisplayString(x_display);
        CStr::from_ptr(p).to_string_lossy().into_owned()
        // ^ into_owned() is critical here to clone the C string
        // before closing the display.
    };
    x::XCloseDisplay(x_display);
    trace!("Closed X Display `{}`", name);
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
            CStr::from_ptr(x::XDisplayString(x_display)).to_string_lossy()
        };
        if x_display.is_null() {
            return failed(format!("Failed to open X display `{}`", actual_name));
        }
        trace!("Opened X Display `{}`", actual_name);
        unsafe {
            match Self::from_xlib_display(x_display) {
                Ok(s) => Ok(s),
                Err(e) => {
                    close_x_display(x_display);
                    Err(e)
                },
            }
        }
    }

    pub unsafe fn from_xlib_display(x_display: *mut x::Display) -> Result<Self> {
        assert_ne!(x_display, ptr::null_mut());

        let screen_count      = x::XScreenCount(x_display);
        let protocol_version  = x::XProtocolVersion(x_display);
        let protocol_revision = x::XProtocolRevision(x_display);
        let vendor_release    = x::XVendorRelease(x_display);
        let server_vendor     = CStr::from_ptr(x::XServerVendor(x_display)).to_string_lossy();
        trace!("X protocol version {}, revision {}", protocol_version, protocol_revision);
        trace!("X server vendor: `{}`, release {}", server_vendor, vendor_release);
        trace!("X server screen count: {}", screen_count);

        let atoms = atoms::PreloadedAtoms::load(x_display)?;
        let invisible_x_cursor = super::cursor::create_invisible_x_cursor(x_display);
        let default_x_cursor = super::cursor::create_default_x_cursor(x_display);
        let xrender = super::xrender::XRender::query(x_display);
        let xi = super::xi::XI::query(x_display);

        let xim = {
            let (db, res_name, res_class) = (ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
            let xim = x::XOpenIM(x_display, db, res_name, res_class);
            if xim.is_null() {
                warn!("XOpenIM() returned NULL");
                None
            } else {
                trace!("Opened XIM {:?}", xim);
                Some(xim)
            }
        };
        let c = X11SharedContext {
            x_display, xim, atoms, xrender, xi, invisible_x_cursor, default_x_cursor,
        };
        Ok(X11Context(Rc::new(c)))
    }
}


impl X11SharedContext {
    pub fn x_default_screen(&self) -> *mut x::Screen {
        unsafe {
            x::XDefaultScreenOfDisplay(self.x_display)
        }
    }
    pub fn x_default_screen_num(&self) -> c_int {
        unsafe {
            x::XDefaultScreen(self.x_display)
        }
    }
    pub fn x_default_root_window(&self) -> x::Window {
        unsafe {
            x::XDefaultRootWindow(self.x_display)
        }
    }
    pub fn x_default_visual(&self) -> *mut x::Visual {
        unsafe {
            x::XDefaultVisual(self.x_display, self.x_default_screen_num())
        }
    }
    /// XFlush() flushes the output buffer.
    pub fn x_flush(&self) {
        unsafe {
            x::XFlush(self.x_display);
        }
    }
    /// XSync() is like XFlush() except that it also waits for all requests
    /// to be processed. In particular, this ensures that the error handler is
    /// called before going any further.
    pub fn x_sync(&self) {
        unsafe {
            x::XSync(self.x_display, x::False);
        }
    }
    #[allow(dead_code)]
    fn x_sync_discarding_all_events_in_the_queue(&self) {
        unsafe {
            x::XSync(self.x_display, x::True);
        }
    }

    fn root_prop<T: PropElement>(&self, prop: x::Atom, req_type: PropType, long_range: Range<usize>) -> Result<PropData<T>> {
        prop::get(self.x_display, self.x_default_root_window(), prop, req_type, long_range)
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
                name: match names.get(i) {
                    None => None,
                    Some(name) => match name {
                        Err(_) => None,
                        Ok(name) => Some(name.clone()), // PERF: Could we avoid this clone()???
                    }
                },
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
            x::XUngrabPointer(self.x_display, x::CurrentTime);
        }
        Ok(())
    }
}

