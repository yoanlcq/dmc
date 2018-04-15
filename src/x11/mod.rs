// TODO WISH: XInitThreads() for multithreading support

extern crate x11;
extern crate libc;

use std::os::raw::c_int;
use self::x11::xlib as x;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XIDeviceId {
    XIMaster(c_int),
    XISlave(c_int),
}
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum X11DeviceId {
    Main,
    XI(XIDeviceId),
}
pub type X11MasterHidId = X11DeviceId;
pub type X11MouseId = X11DeviceId;
pub type X11KeyboardId = X11DeviceId;
pub type X11TabletId = XIDeviceId;
pub type X11TouchId = XIDeviceId;
pub type X11Keysym = x::KeySym;
pub type X11Keycode = x::KeyCode;


pub mod context;
pub use self::context::{X11Context, X11SharedContext};
pub mod window;
pub use self::window::{X11Window, X11SharedWindow, X11WindowHandle, X11WindowFromHandleParams};
pub mod cursor;
pub use self::cursor::{X11Cursor, X11SharedCursor};
pub mod event;
pub mod gl;
pub use self::gl::{
    X11GLPixelFormat,
    X11GLContext,
    X11GLProc,
};
pub mod glx;
pub mod xrender;
pub mod xi;
pub mod atoms;
pub mod prop;
pub mod xlib_error;
pub mod missing_bits;
pub mod net_wm;
pub mod motif_wm;
pub mod keys;

