use std::sync::atomic::{ATOMIC_BOOL_INIT, AtomicBool, Ordering};

static mut G_XLIB_ERROR_OCCURED: AtomicBool = ATOMIC_BOOL_INIT;
static mut G_GLX_ERROR_BASE: i32 = 0;
static mut G_GLX_EVENT_BASE: i32 = 0;
static mut G_XRENDER_ERROR_BASE: i32 = 0;
static mut G_XRENDER_EVENT_BASE: i32 = 0;
static mut G_XI_ERROR_BASE: i32 = 0;
static mut G_XI_EVENT_BASE: i32 = 0;
static mut G_XI_OPCODE: i32 = 0;

// WISH: Grab from _XPrintDefaultError in Xlib's sources
pub unsafe extern fn xlib_generic_error_handler(_shared_context: *mut x::Display, e: *mut x::XErrorEvent) -> c_int {
    // NOTE: DO NOT make requests to the X server within X error handlers such as this one.
    G_XLIB_ERROR_OCCURED.store(true, Ordering::SeqCst);
    let e = *e;
    error!("Received X error: XErrorEvent {{ type: {}, display: {:?}, resourceid: {}, serial: {}, error_code: {}, request_code: {}, minor_code: {} }}", e.type_, e.display, e.resourceid, e.serial, e.error_code, e.request_code, e.minor_code);
    0
}


