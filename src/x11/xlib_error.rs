// Useful reference: _XPrintDefaultError() in XlibInt.c (X11 sources)

use std::os::raw::c_int;
use std::ffi::CStr;
use super::x11::xlib as x;
use error::{Result, failed};

fn swapping_error_handlers<T, F: FnMut() -> T>(mut f: F) -> T {
    unsafe {
        let previous_error_handler = x::XSetErrorHandler(Some(our_xlib_error_handler));
        let previous_io_error_handler = x::XSetIOErrorHandler(Some(our_xlib_io_error_handler));
        let out = f();
        x::XSetErrorHandler(previous_error_handler);
        x::XSetIOErrorHandler(previous_io_error_handler);
        out
    }
}

unsafe fn syncing<T, F: FnMut() -> T>(x_display: *mut x::Display, mut f: F) -> T {
    assert!(!x_display.is_null());
    x::XSync(x_display, x::False);
    let out = f();
    x::XSync(x_display, x::False);
    out
}

pub static DO_USE_DMC_XLIB_ERROR_HANDLERS: bool = true;

pub unsafe fn sync_catch<T, F: FnMut() -> T>(x_display: *mut x::Display, mut f: F) -> Result<T> {
    assert!(!x_display.is_null());
    let out = if DO_USE_DMC_XLIB_ERROR_HANDLERS {
        swapping_error_handlers(|| syncing(x_display, || f()))
    } else {
        syncing(x_display, f)
    };
    match ERROR_EVENT.take() {
        None => Ok(out),
        Some(x::XErrorEvent {
            type_: _, display: _,
            resourceid, serial, error_code,
            request_code, minor_code,
        }) => {
            let mut buf = [0_u8; 1024];
            x::XGetErrorText(x_display, error_code as _, buf.as_mut_ptr() as _, buf.len() as _);
            let error_text = CStr::from_ptr(buf.as_ptr() as _).to_string_lossy().into_owned();
            let request_text = if request_code < 128 {
                let number = format!("{}", request_code);
                x::XGetErrorDatabaseText(x_display, "XRequest\0".as_ptr() as _, number.as_ptr() as _, "\0".as_ptr() as _, buf.as_mut_ptr() as _, buf.len() as _);
                CStr::from_ptr(buf.as_ptr() as _).to_string_lossy().into_owned()
            } else {
                match super::context::ALL_EXTENSIONS.as_ref() {
                    None => {
                        error!("ALL_EXTENSIONS global was not initialized!");
                        "???".to_owned()
                    },
                    Some(exts) => {
                        let found_name = exts.get(&(request_code as _)).map(|ext| ext.name.clone());
                        found_name.unwrap_or_else(|| "???".to_owned())
                    }
                }
            };

            failed(format!("X Error {}: {} (resourceid: {}, serial: {}, request_code: {} ({}), minor_code: {})", error_code, error_text, resourceid, serial, request_code, request_text, minor_code))
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

