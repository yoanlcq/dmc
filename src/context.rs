//! The `Context` structure, which is also the main entry point for this crate.

use os::OsContext;
use error::Result;

/// Common and globally needed platform-specific data.
/// This is the entry point for creating various objects such as `Window`s and `GLContext`s.
#[derive(Debug)]
pub struct Context(pub(crate) OsContext);

#[cfg(nightly)]
mod nightly {
    use super::Context;
    impl !Send for Context {}
    impl !Sync for Context {}
}

impl Context {
    /// Attempts to get one handle to the platform-specific display backend.
    /// 
    /// You must assume that it is invalid to have more than one Context at a time (it's also pointless).  
    /// It may work on some platforms (because we can make it so), but otherwise this could cause failures
    /// at various points.
    pub fn new() -> Result<Self> {
        OsContext::new().map(Context)
    }
    /// Undoes any mouse trap caused by any window.
    pub fn untrap_mouse(&self) -> Result<()> {
        self.0.untrap_mouse()
    }
}
