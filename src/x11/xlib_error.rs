use std::os::raw::c_int;
use std::ffi::CStr;
use super::x11::xlib as x;
use error::{Result, failed};

pub fn sync_catch<T, F: FnMut() -> T>(x_display: *mut x::Display, mut f: F) -> Result<T> {
    let out = unsafe {
        let previous_error_handler = x::XSetErrorHandler(Some(our_xlib_error_handler));
        let previous_io_error_handler = x::XSetIOErrorHandler(Some(our_xlib_io_error_handler));
        let out = f();
        x::XSync(x_display, x::False);
        x::XSetErrorHandler(previous_error_handler);
        x::XSetIOErrorHandler(previous_io_error_handler);
        out
    };
    match unsafe { ERROR_EVENT }.take() {
        None => Ok(out),
        Some(x::XErrorEvent {
            type_: _, display: _,
            resourceid, serial, error_code,
            request_code, minor_code,
        }) => {
            let mut buf = [0_u8; 2048];
            unsafe {
                x::XGetErrorText(x_display, error_code as _, buf.as_mut_ptr() as *mut _, buf.len() as _);
            }
            let text = CStr::from_bytes_with_nul(&buf).unwrap().to_string_lossy();
            failed(format!("X Error {}: {} (resourceid: {}, serial: {}, request_code: {}, minor_code: {})", error_code, text, resourceid, serial, request_code, minor_code))
        },
    }
}

static mut ERROR_EVENT: Option<x::XErrorEvent> = None;

extern fn our_xlib_error_handler(_x_display: *mut x::Display, e: *mut x::XErrorEvent) -> c_int {
    unsafe {
        ERROR_EVENT = Some(*e);
    }
    0 // The return value is ignored anyway
}
extern fn our_xlib_io_error_handler(_x_display: *mut x::Display) -> c_int {
    // See the man page for XSetIOErrorHandler().
    panic!("An I/O error occured in Xlib. Xlib is supposed to force the process to exit now, so we'll panic instead.")
}

