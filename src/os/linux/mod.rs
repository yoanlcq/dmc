extern crate libevdev_sys;
extern crate libudev_sys;

use x11::{X11Context, X11Window};
pub use x11::{
    X11GLPixelFormat as OsGLPixelFormat, 
    X11GLContext     as OsGLContext, 
    X11GLProc        as OsGLProc,
};

use error::Result;
use window::WindowSettings;

#[derive(Debug)]
pub struct OsContext {
    pub x11: X11Context,
}

#[derive(Debug)]
pub struct OsWindow(pub X11Window);

impl From<X11Context> for OsContext {
    fn from(x11: X11Context) -> Self {
        Self { x11 }
    }
}

impl OsContext {
    pub fn new() -> Result<Self> {
        X11Context::new().map(Self::from)
    }
    pub fn create_window(&self, window_settings: &WindowSettings) -> Result<OsWindow> {
        self.x11.create_window(window_settings).map(OsWindow)
    }
}
