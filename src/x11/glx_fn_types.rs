use std::os::raw::c_int;
use super::x11::xlib as x;
use super::x11::glx::*;
use super::x11::glx::arb::*;


#![allow(non_camel_case_types)]
pub type glXGetProcAddress = unsafe extern fn(*const u8) -> Option<unsafe extern fn()>;
pub type glXSwapIntervalMESA = unsafe extern fn(interval: c_int) -> c_int;
pub type glXGetSwapIntervalMESA = unsafe extern fn() -> c_int;
pub type glXSwapIntervalSGI = unsafe extern fn(interval: c_int) -> c_int;
pub type glXSwapIntervalEXT = unsafe extern fn(
    *mut x::Display, GLXDrawable, interval: c_int
);
pub type glXCreateContextAttribsARB = unsafe extern fn(
    *mut x::Display, GLXFBConfig, share_context: GLXContext, 
    direct: x::Bool, attrib_list: *const c_int
) -> GLXContext;

