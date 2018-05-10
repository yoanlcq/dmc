mod linuxdev;

extern crate x11;
extern crate libc;

use std::time::Instant;
use std::os::raw::c_int;
use std::collections::HashMap;

use self::linuxdev::{LinuxdevContext, LinuxdevToken};
pub use self::linuxdev::{OsControllerInfo, OsControllerState};
use x11::{
    X11Context, X11Window, X11WindowHandle, X11WindowFromHandleParams, X11Cursor,
    X11GLProc, X11GLPixelFormat, X11GLContext,
    X11Keysym, X11Keycode,
};
use error::{Result};
use desktop::Desktop;
use window::WindowSettings;
use event::Event;
use timeout::Timeout;
use device::{
    self,
    DeviceID, DeviceInfo, ButtonState,
    ControllerButton, ControllerAxis, ControllerState, VibrationState,
    KeyState, KeyboardState, Keysym, Keycode,
    MouseState, MouseButton,
    TabletInfo, TabletState, TabletPadButton, TabletStylusButton,
    TouchInfo,
};
use cursor::{SystemCursor, RgbaCursorData, RgbaCursorAnimFrame};
use Extent2;

#[derive(Debug)]
pub struct OsContext {
    pub x11: X11Context,
    pub linuxdev: LinuxdevContext,
}

pub type OsWindow = X11Window;
pub type OsWindowHandle = X11WindowHandle;
pub type OsWindowFromHandleParams = X11WindowFromHandleParams;
pub type OsCursor = X11Cursor;
pub type OsGLPixelFormat = X11GLPixelFormat;
pub type OsGLContext = X11GLContext;
pub type OsGLProc = X11GLProc;
pub type OsKeycode = X11Keycode;
pub type OsKeysym = X11Keysym;

pub mod event_instant;
pub use self::event_instant::OsEventInstant;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum OsDeviceID {
    CoreKeyboard,
    CorePointer,
    XISlave(c_int),
    Linuxdev(LinuxdevToken),
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsKeyboardState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsMouseButtonsState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsTabletPadButtonsState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsTabletStylusButtonsState;


impl OsKeyboardState {
    pub fn keycode(&self, key: Keycode) -> Option<KeyState> {
        unimplemented!{}
    }
    pub fn keysym(&self, key: Keysym) -> Option<KeyState> {
        unimplemented!{}
    }
}
impl OsMouseButtonsState {
    pub fn button(&self, button: MouseButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}
impl OsTabletPadButtonsState {
    pub fn button(&self, button: TabletPadButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}
impl OsTabletStylusButtonsState {
    pub fn button(&self, button: TabletStylusButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}



impl From<X11Context> for OsContext {
    fn from(x11: X11Context) -> Self {
        Self { x11, linuxdev: LinuxdevContext::default(), }
    }
}

impl OsContext {
    pub fn new() -> Result<Self> {
        X11Context::new().map(Self::from)
    }
    pub fn create_window(&self, window_settings: &WindowSettings) -> Result<OsWindow> {
        self.x11.create_window(window_settings)
    }
    pub fn window_from_handle(&self, handle: OsWindowHandle, params: Option<&OsWindowFromHandleParams>) -> Result<OsWindow> {
        self.x11.window_from_handle(handle, params)
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
    fn poll_next_event(&self) -> Option<Event> {
        self.linuxdev.poll_next_event().or_else(|| self.x11.poll_next_event())
    }
    pub fn next_event(&self, timeout: Timeout) -> Option<Event> {
        match timeout.duration() {
            None => loop {
                if let Some(e) = self.poll_next_event() {
                    return Some(e);
                }
            },
            Some(duration) => {
                // Welp, just poll repeatedly instead, until the day we care to
                // implement a more sophisticated technique
                let start = Instant::now();
                loop {
                    if let Some(e) = self.poll_next_event() {
                        return Some(e);
                    }
                    if start.elapsed() >= duration {
                        return None; // Timed out
                    }
                }
            },
        }
    }
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        self.x11.supports_raw_device_events()
    }
    pub fn devices(&self) -> device::Result<HashMap<DeviceID, DeviceInfo>> {
        match (self.x11.devices(), self.linuxdev.controllers()) {
            (Ok(mut devs), Ok(controllers)) => Ok({
                devs.extend(controllers.into_iter());
                devs
            }),
            // Honestly, none of these should happen; at worst, we should get empty HashMaps.
            (Err(x11), Err(controllers)) => {
                device::failed(format!("Could not get devices via X11 and udev: respective errors are:\n- X11: {}\n- udev: {}", x11, controllers))
            },
            (Ok(x11), Err(controllers)) => {
                error!("Could not get devices via udev: {}", controllers);
                Ok(x11)
            },
            (Err(x11), Ok(controllers)) => {
                error!("Could not get devices via X11: {}", x11);
                Ok(controllers)
            },
        }
    }
    pub fn ping_device(&self, id: DeviceID) -> device::Result<()> {
        match id.0 {
            OsDeviceID::Linuxdev(token) => self.linuxdev.ping_controller(token),
            _ => unimplemented!{},
        }
    }
    pub fn controller_state(&self, controller: DeviceID) -> device::Result<ControllerState> {
        self.linuxdev.controller_state(controller)
    }
    pub fn controller_button_state(&self, controller: DeviceID, button: ControllerButton) -> device::Result<ButtonState> {
        self.linuxdev.controller_button_state(controller, button)
    }
    pub fn controller_axis_state(&self, controller: DeviceID, axis: ControllerAxis) -> device::Result<f64> {
        self.linuxdev.controller_axis_state(controller, axis)
    }
    pub fn controller_set_vibration(&self, controller: DeviceID, vibration: &VibrationState) -> device::Result<()> {
        self.linuxdev.controller_set_vibration(controller, vibration)
    }
    pub fn main_mouse(&self) -> device::Result<DeviceID> {
        Ok(self.x11.core_x_mouse())
    }
    pub fn main_keyboard(&self) -> device::Result<DeviceID> {
        Ok(self.x11.core_x_keyboard())
    }
    pub fn keyboard_state(&self, keyboard: DeviceID) -> device::Result<KeyboardState> {
        unimplemented!{}
    }
    pub fn keyboard_keycode_state(&self, keyboard: DeviceID, keycode: Keycode) -> device::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keyboard_keysym_state(&self, keyboard: DeviceID, keysym: Keysym) -> device::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keysym_name(&self, keysym: Keysym) -> device::Result<String> {
        unimplemented!{}
    }
    pub fn keysym_from_keycode(&self, keyboard: DeviceID, keycode: Keycode) -> device::Result<Keysym> {
        unimplemented!{}
    }
    pub fn keycode_from_keysym(&self, keyboard: DeviceID, keysym: Keysym) -> device::Result<Keycode> {
        unimplemented!{}
    }
    pub fn mouse_state(&self, mouse: DeviceID) -> device::Result<MouseState> {
        unimplemented!{}
    }
    pub fn tablet_state(&self, tablet: DeviceID) -> device::Result<TabletState> {
        unimplemented!{}
    }
}
