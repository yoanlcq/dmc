//! Internal module for all X11 things.
//!
//! # F.A.Q (for posterity)
//!
//! ## Why isn't this under `os`?
//!
//! Because X11 is not an OS; it is a backend that makes sense for
//! Linux and BSDs.
//!
//!
//! ## Why use Xlib (instead of e.g XCB)?
//!
//! Because as of this writing (May 2018), [`rust-xcb`](https://github.com/rtbo/rust-xcb)
//! doesn't have XInput2 bindings (there's a [pull request](https://github.com/rtbo/rust-xcb/pull/33)
//! to address this, but it looks like it went inactive long ago.)
//!
//! I could have used the raw XCB API instead, but it's a chore to use as-is, which
//! is why `rust-xcb` exists in the first place. I decided to settle with Xlib and roll
//! with it, even though the way it handles errors sucks.
//!
//!
//! ## When should `xlib_error::sync_catch()` be called?
//!
//! In as many places as possible. If in doubt, use it.
//!
//! Xlib errors are deadly: They don't provide detailed explanations of what
//! did go wrong and when. The default Xlib error handlers terminate the program, and
//! finally, we might never see an error until somebody calls `XSync()` or pumps events.
//!
//! Yes, `XSync()` is "slow", but it's not me who decided that window functionality was to be
//! provided by some server via some protocol. Especially given the (very bad) way Xlib handles
//! error (this is not X11's fault per se), I think the cost of `XSync()` is very small compared
//! to the cost of fixing weird, undebuggable and unreproducible issues reported by users, caused by quirky setups
//! or some actual bug in our code that triggers under rare conditions; because that's how it is
//! with Xlib.
//!
//!
//! ## Why not use `XSynchronize()` to turn on synchronous behaviour by default?
//!
//! [This page](https://tronche.com/gui/x/xlib/event-handling/protocol-errors/synchronization.html)
//! says "Note that graphics may occur 30 or more times more slowly when synchronization is enabled.".  
//! This is probably not relevant anymore today but it's still scary if one day we want to provide
//! software rendering via standard Xlib drawing functions.

extern crate x11;
extern crate libc;

use self::x11::xlib as x;

pub type X11Keysym = x::KeySym;
pub type X11Keycode = x::KeyCode;

pub mod context;
pub use self::context::{X11Context, X11SharedContext};
pub mod window;
pub use self::window::{X11Window, X11SharedWindow, X11WindowHandle, X11WindowFromHandleParams};
pub mod cursor;
pub use self::cursor::{X11Cursor, X11SharedCursor};
pub mod event;
pub use self::event::X11UnprocessedEvent;
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

use hint::Hint;
use error::{Result, failed};

pub fn set_hint(hint: Hint) -> Result<()> {
    match hint {
        Hint::XlibXInitThreads => match unsafe { x::XInitThreads() } {
            0 => failed("XInitThreads() failed"),
            _ => Ok(()),
        },
        Hint::XlibDefaultErrorHandlers(use_xlib) => unsafe {
            xlib_error::DO_USE_DMC_XLIB_ERROR_HANDLERS = !use_xlib;
            Ok(())
        },
    }
}
