//! Hints affect this crate's behaviour globally.
//!
//! XXXXXXX

use error::Result;
use os;

pub fn set_hint(hint: Hint) -> Result<()> {
    os::set_hint(hint)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Hint {
    /// Calls `XInitThreads()`, which allows using Xlib from multiple threads.  
    /// By default, it is not called.  
    /// Setting this hint only works **BEFORE** any call to Xlib is made.
    XlibXInitThreads,
    /// If `false`, this crate's Xlib error handlers are used. Otherwise, they aren't. Defaults to
    /// `false` and can be called any time, but only from the main thread.
    XlibDefaultErrorHandlers(bool),
}
