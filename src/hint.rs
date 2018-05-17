//! Hints affect this crate's behaviour globally.

use error::Result;
use os;

/// Sets a global hint for this crate.
/// 
/// Such global hints should hopefully be rare, but sometimes they
/// are the proper way to deal with some platform-specific details
/// (usually, they are global in the first place because of quirky lower-level APIs).
/// 
/// The appropriate time and place to call this function depends on
/// the `Hint` variant you want to set; it is explained in the variant's documentation.
/// 
/// Most hint are platform-specific; this function only returns `Ok(())` when
/// the hint is indeed supported and setting it has "succeeded", whatever it means for that particular case.  
/// If a hint does not make sense for the current platform, expect this function to fail with `ErrorKind::Unsupported`.
pub fn set_hint(hint: Hint) -> Result<()> {
    os::set_hint(hint)
}

/// A global hint for use by this crate.
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
