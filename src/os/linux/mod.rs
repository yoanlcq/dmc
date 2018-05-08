mod udev;

extern crate x11;
extern crate libc;

use std::time::Instant;

use self::udev::{UdevContext, TokenForUdev};
pub use self::udev::{OsControllerInfo, OsControllerState};
use x11::{
    X11Context, X11Window, X11WindowHandle, X11WindowFromHandleParams, X11Cursor,
    X11GLProc, X11GLPixelFormat, X11GLContext,
    X11Keysym, X11Keycode,
    X11DeviceID,
};
use error::Result;
use desktop::Desktop;
use window::WindowSettings;
use event::Event;
use timeout::Timeout;
use hid::{
    self, 
    HidID, HidInfo, ButtonState,
    ControllerButton, ControllerAxis, ControllerState, ControllerInfo, RumbleEffect,
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
    pub udev: UdevContext,
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
pub struct OsHidID {
    pub x11: Option<X11DeviceID>,
    pub token_for_udev: Option<TokenForUdev>,
}

impl From<X11DeviceID> for OsHidID {
    fn from(x11: X11DeviceID) -> Self {
        Self {
            x11: Some(x11),
            token_for_udev: None,
        }
    }
}
impl From<TokenForUdev> for OsHidID {
    fn from(token: TokenForUdev) -> Self {
        Self {
            x11: None,
            token_for_udev: Some(token),
        }
    }
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
        Self { x11, udev: UdevContext::default(), }
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
        if let Some(e) = self.udev.poll_next_event() {
            return Some(e);
        }
        if let Some(e) = self.x11.poll_next_event() {
            return Some(e);
        }
        None
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
                    if Instant::now().duration_since(start) >= duration {
                        return None; // Timed out
                    }
                }
            },
        }
    }
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        self.x11.supports_raw_device_events()
    }
    pub fn hid_info(&self, id: HidID) -> hid::Result<HidInfo> {
        self.udev.hid_info(id)
    }
    pub fn ping_hid(&self, id: HidID) -> hid::Result<()> {
        self.udev.ping_hid(id)
    }
    pub fn controllers(&self) -> hid::Result<Vec<HidID>> {
        self.udev.controllers()
    }
    pub fn controller_state(&self, controller: HidID) -> hid::Result<ControllerState> {
        self.udev.controller_state(controller)
    }
    pub fn controller_button_state(&self, controller: HidID, button: ControllerButton) -> hid::Result<ButtonState> {
        self.udev.controller_button_state(controller, button)
    }
    pub fn controller_axis_state(&self, controller: HidID, axis: ControllerAxis) -> hid::Result<f64> {
        self.udev.controller_axis_state(controller, axis)
    }
    pub fn controller_play_rumble_effect(&self, controller: HidID, effect: &RumbleEffect) -> hid::Result<()> {
        self.udev.controller_play_rumble_effect(controller, effect)
    }
    pub fn keyboards(&self) -> hid::Result<Vec<HidID>> {
        unimplemented!{}
    }
    pub fn main_keyboard(&self) -> hid::Result<HidID> {
        Ok(self.x11.core_x_keyboard())
    }
    pub fn keyboard_state(&self, keyboard: HidID) -> hid::Result<KeyboardState> {
        unimplemented!{}
    }
    pub fn keyboard_keycode_state(&self, keyboard: HidID, keycode: Keycode) -> hid::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keyboard_keysym_state(&self, keyboard: HidID, keysym: Keysym) -> hid::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keysym_name(&self, keysym: Keysym) -> hid::Result<String> {
        unimplemented!{}
    }
    pub fn keysym_from_keycode(&self, keyboard: HidID, keycode: Keycode) -> hid::Result<Keysym> {
        unimplemented!{}
    }
    pub fn keycode_from_keysym(&self, keyboard: HidID, keysym: Keysym) -> hid::Result<Keycode> {
        unimplemented!{}
    }
    pub fn mice(&self) -> hid::Result<Vec<HidID>> {
        unimplemented!{}
    }
    pub fn main_mouse(&self) -> hid::Result<HidID> {
        Ok(self.x11.core_x_mouse())
    }
    pub fn mouse_state(&self, mouse: HidID) -> hid::Result<MouseState> {
        unimplemented!{}
    }
    pub fn tablets(&self) -> hid::Result<Vec<HidID>> {
        unimplemented!{}
    }
    pub fn tablet_state(&self, tablet: HidID) -> hid::Result<TabletState> {
        unimplemented!{}
    }
    pub fn touch_devices(&self) -> hid::Result<Vec<HidID>> {
        unimplemented!{}
    }
}
