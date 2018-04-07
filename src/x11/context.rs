use std::ptr;
use std::ffi::CStr;
use std::rc::Rc;
use std::ops::Deref;
use context::Context;
use error::{Result, failed};
use os::OsContext;
use super::x11::xlib as x;
use super::atoms;


/// On X11-based targets, a `Context` **owns** an Xlib `Display` pointer.
impl Context {
    /// X11-only specialization of `new()` where you can specify
    /// the name given to `XOpenDisplay()`.
    pub fn with_x11_display_name(name: Option<&CStr>) -> Result<Self> {
        X11Context::with_x11_display_name(name).map(OsContext::from).map(Context)
    }
    /// X11-only specialization of `new()` where you **transfer ownership**
    /// of an existing Xlib `Display` pointer.
    pub fn with_xlib_display(dpy: *mut x::Display) -> Result<Self> {
        X11Context::with_xlib_display(dpy).map(OsContext::from).map(Context)
    }
    /// (X11-only) Gets the `Display` pointer associated with this `Context`.
    ///
    /// Be careful: It is closed when the `Context` is dropped.
    pub fn xlib_display(&self) -> *mut x::Display {
        self.0.x11.x_display
    }
}



#[derive(Debug)]
pub struct X11Context(pub Rc<X11SharedContext>);

#[derive(Debug)]
pub struct X11SharedContext {
    pub x_display: *mut x::Display,
    pub xim: Option<x::XIM>,
    pub atoms: atoms::PreloadedAtoms,
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
            x_display, xim, atoms: _,
        } = self;
        unsafe {
            if let Some(xim) = xim {
                x::XCloseIM(xim);
            }
            let name = {
                let p = x::XDisplayString(x_display);
                CStr::from_ptr(p).to_string_lossy().into_owned()
                // ^ into_owned() is critical here to clone the C string
                // before closing the display.
            };
            x::XCloseDisplay(x_display);
            info!("Closed X Display `{}`", name);
        }
    }
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
        if x_display.is_null() {
            let actual_name = unsafe {
                let p = x::XDisplayString(x_display);
                CStr::from_ptr(p).to_string_lossy()
            };
            return failed(format!("Failed to open X display `{}`", actual_name));
        }
        Self::with_xlib_display(x_display)
    }

    pub fn with_xlib_display(x_display: *mut x::Display) -> Result<Self> {
        assert_ne!(x_display, ptr::null_mut());
        // NOTE: No need to free it
        let actual_name = unsafe {
            let p = x::XDisplayString(x_display);
            CStr::from_ptr(p).to_string_lossy()
        };
        info!("Opened X Display `{}`", actual_name);
        let xim = {
            let xim = unsafe {
                x::XOpenIM(x_display, ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
            };
            if xim.is_null() {
                None
            } else {
                Some(xim)
            }
        };
        let atoms = atoms::PreloadedAtoms::load(x_display);
        let c = X11SharedContext {
            x_display, xim, atoms,
        };
        Ok(X11Context(Rc::new(c)))
    }
}
