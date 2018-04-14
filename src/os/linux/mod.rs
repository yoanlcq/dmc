extern crate libevdev_sys;
extern crate libudev_sys;

use x11::{
    X11Context, X11Window, X11WindowHandle, X11Cursor,
    X11GLProc, X11GLPixelFormat, X11GLContext,
};
use error::Result;
use desktop::Desktop;
use window::WindowSettings;
use event::Event;
use timeout::Timeout;
use cursor::{SystemCursor, RgbaCursorData, RgbaCursorAnimFrame};
use Extent2;

#[derive(Debug)]
pub struct OsContext {
    pub x11: X11Context,
    // NOTE: Later, udev and evdev members.
}

pub type OsWindow = X11Window;
pub type OsWindowHandle = X11WindowHandle;
pub type OsCursor = X11Cursor;
pub type OsGLPixelFormat = X11GLPixelFormat;
pub type OsGLContext = X11GLContext;
pub type OsGLProc = X11GLProc;


pub type OsMasterHidId = i32;
pub type OsControllerId = i32;
pub type OsControllerState = i32;
pub type OsKeyboardId = i32;
pub type OsKeyboardState = i32;
pub type OsVKey = i32;
pub type OsMouseId = i32;
pub type OsMouseButtonsState = i32;
pub type OsTabletId = i32;
pub type OsTabletPadButtonsState = i32;
pub type OsTabletStylusButtonsState = i32;
pub type OsTouchId = i32;



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
        self.x11.create_window(window_settings)
    }
    pub fn create_window_from_handle(&self, handle: OsWindowHandle) -> Result<OsWindow> {
        self.x11.create_window_from_handle(handle)
    }
    pub fn desktops(&self) -> Result<Vec<Desktop>> {
        self.x11.desktops()
    }
    pub fn current_desktop(&self) -> Result<usize> {
        self.x11.current_desktop()
    }
    pub fn create_system_cursor(&self, s: SystemCursor) -> Result<OsCursor> {
        self.x11.create_system_cursor(s)
    }
    pub fn best_cursor_size(&self, size_hint: Extent2<u32>) -> Result<Extent2<u32>> {
        self.x11.best_cursor_size(size_hint)
    }
    pub fn create_rgba_cursor(&self, data: &RgbaCursorData) -> Result<OsCursor> {
        self.x11.create_rgba_cursor(data)
    }
    pub fn create_animated_rgba_cursor(&self, frames: &[RgbaCursorAnimFrame]) -> Result<OsCursor> {
        self.x11.create_animated_rgba_cursor(frames)
    }
    pub fn untrap_mouse(&self) -> Result<()> {
        self.x11.untrap_mouse()
    }
    pub fn next_event(&self, timeout: Timeout) -> Option<Event> {
        self.x11.next_event(timeout)
    }
}
