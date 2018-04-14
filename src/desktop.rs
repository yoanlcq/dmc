//! Getting information about the user's desktop(s).

use context::Context;
use error::Result;
use Rect;

impl Context {
    /// Gets the user's desktops.
    ///
    /// On some platforms, there is only one.
    pub fn desktops(&self) -> Result<Vec<Desktop>> {
        self.0.desktops()
    }
    /// Gets the current desktop as an index in the array of desktops.
    pub fn current_desktop(&self) -> Result<usize> {
        self.0.current_desktop()
    }
}

/// Data associated with a desktop.
#[derive(Debug, Clone)]
pub struct Desktop {
    /// The desktop's name, if any.
    pub name: Option<String>,
    /// The desktop's work area, that is, the advised zone that excludes
    /// panels, tasks bars etc.
    pub work_area: Rect<i32, u32>,
}
