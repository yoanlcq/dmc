mod udev;

use std::time::Instant;
use self::udev::{UdevContext, UdevDeviceId};
use x11::{
    X11Context, X11Window, X11WindowHandle, X11WindowFromHandleParams, X11Cursor,
    X11GLProc, X11GLPixelFormat, X11GLContext,
    X11Keysym, X11Keycode,
    X11KeyboardId, X11MouseId, X11TabletId, X11TouchId, X11MasterHidId,
};
use error::Result;
use desktop::Desktop;
use window::WindowSettings;
use event::Event;
use timeout::Timeout;
use hid::{
    self, 
    AnyDeviceId, HidInfo, AxisInfo, ButtonState,
    ControllerButton, ControllerAxis, ControllerId, ControllerState, ControllerInfo,
    KeyboardId, KeyState, KeyboardState, Keysym, Keycode,
    MouseId, MouseState, MouseButton,
    TabletId, TabletInfo, TabletState, TabletPadButton, TabletStylusButton,
    TouchId, TouchInfo,
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


#[derive(Debug, Clone, PartialEq)]
pub struct OsControllerInfo;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct OsMasterHidId {
    // It's either one or both; Normally the missing one is deduced from the other.
    // It's invalid for them to be both `None`.
    pub x11: Option<X11MasterHidId>,
    pub udev: Option<UdevDeviceId>,
}
pub type OsControllerId = i32; // ??? How to identify via udev ?
pub type OsKeyboardId = X11KeyboardId;
pub type OsMouseId = X11MouseId;
pub type OsTabletId = X11TabletId;
pub type OsTouchId = X11TouchId;
pub type OsKeysym = X11Keysym;
pub type OsKeycode = X11Keycode;

#[derive(Debug, Clone, PartialEq)]
pub struct OsControllerState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsKeyboardState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsMouseButtonsState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsTabletPadButtonsState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsTabletStylusButtonsState;

pub trait OsDeviceId {}

impl OsControllerInfo {
    pub fn has_button(&self, button: ControllerButton) -> bool {
        unimplemented!{}
    }
    pub fn has_axis(&self, axis: ControllerAxis) -> bool {
        unimplemented!{}
    }
    pub fn axis(&self, axis: ControllerAxis) -> Option<AxisInfo> {
        unimplemented!{}
    }
}
impl OsControllerState {
    pub fn button(&self, button: ControllerButton) -> Option<ButtonState> {
        unimplemented!{}
    }
    pub fn axis(&self, axis: ControllerAxis) -> Option<f64> {
        unimplemented!{}
    }
}
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
    pub fn hid_info<Id: AnyDeviceId>(&self, id: Id) -> hid::Result<HidInfo> {
        unimplemented!{}
    }
    pub fn ping_hid<Id: AnyDeviceId>(&self, id: Id) -> hid::Result<()> {
        unimplemented!{}
    }
    pub fn controllers(&self) -> hid::Result<Vec<ControllerId>> {
        self.udev.controllers()
    }
    pub fn controller_info(&self, controller: ControllerId) -> hid::Result<ControllerInfo> {
        self.udev.controller_info(controller)
    }
    pub fn controller_state(&self, controller: ControllerId) -> hid::Result<ControllerState> {
        self.udev.controller_state(controller)
    }
    pub fn controller_button_state(&self, controller: ControllerId, button: ControllerButton) -> hid::Result<ButtonState> {
        self.udev.controller_button_state(controller, button)
    }
    pub fn controller_axis_state(&self, controller: ControllerId, axis: ControllerAxis) -> hid::Result<f64> {
        self.udev.controller_axis_state(controller, axis)
    }
    pub fn keyboards(&self) -> hid::Result<Vec<KeyboardId>> {
        unimplemented!{}
    }
    pub fn main_keyboard(&self) -> hid::Result<KeyboardId> {
        unimplemented!{}
    }
    pub fn keyboard_state(&self, keyboard: KeyboardId) -> hid::Result<KeyboardState> {
        unimplemented!{}
    }
    pub fn keyboard_keycode_state(&self, keyboard: KeyboardId, keycode: Keycode) -> hid::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keyboard_keysym_state(&self, keyboard: KeyboardId, keysym: Keysym) -> hid::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keysym_name(&self, keysum: Keysym) -> hid::Result<String> {
        unimplemented!{}
    }
    pub fn keysym_from_keycode(&self, keyboard: KeyboardId, keycode: Keycode) -> hid::Result<Keysym> {
        unimplemented!{}
    }
    pub fn keycode_from_keysym(&self, keyboard: KeyboardId, keysym: Keysym) -> hid::Result<Keycode> {
        unimplemented!{}
    }
    pub fn mice(&self) -> hid::Result<Vec<MouseId>> {
        unimplemented!{}
    }
    pub fn main_mouse(&self) -> hid::Result<MouseId> {
        unimplemented!{}
    }
    pub fn mouse_state(&self, mouse: MouseId) -> hid::Result<MouseState> {
        unimplemented!{}
    }
    pub fn tablets(&self) -> hid::Result<Vec<TabletId>> {
        unimplemented!{}
    }
    pub fn tablet_info(&self, tablet: TabletId) -> hid::Result<TabletInfo> {
        unimplemented!{}
    }
    pub fn tablet_state(&self, tablet: TabletId) -> hid::Result<TabletState> {
        unimplemented!{}
    }
    pub fn touch_devices(&self) -> hid::Result<Vec<TouchId>> {
        unimplemented!{}
    }
    pub fn touch_info(&self, touch: TouchId) -> hid::Result<TouchInfo> {
        unimplemented!{}
    }
}
