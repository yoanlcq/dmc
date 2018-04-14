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
use hid::{
    self, 
    DeviceId, HidInfo, AxisInfo, ButtonState,
    ControllerButton, ControllerAxis, ControllerId, ControllerState, ControllerInfo,
    KeyboardId, KeyState, KeyboardState, ScanCode, KeyCode,
    MouseId, MouseState, MouseButton,
    TabletId, TabletInfo, TabletState, TabletPadButton, TabletStylusButton,
    TouchId, TouchInfo,
};
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


#[derive(Debug, Clone, PartialEq)]
pub struct OsControllerInfo;

pub type OsMasterHidId = i32;
pub type OsControllerId = i32;
pub type OsKeyboardId = i32;
pub type OsMouseId = i32;
pub type OsTabletId = i32;
pub type OsTouchId = i32;
pub type OsScanCode = i32;
pub type OsKeyCode = i32;

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
    pub fn key(&self, key: ScanCode) -> Option<KeyState> {
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
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        self.x11.supports_raw_device_events()
    }
    pub fn hid_info<Id: DeviceId>(&self, id: Id) -> hid::Result<HidInfo> {
        unimplemented!{}
    }
    pub fn ping_hid<Id: DeviceId>(&self, id: Id) -> hid::Result<()> {
        unimplemented!{}
    }
    pub fn controllers(&self) -> hid::Result<Vec<ControllerId>> {
        unimplemented!{}
    }
    pub fn controller_info(&self, controller: ControllerId) -> hid::Result<ControllerInfo> {
        unimplemented!{}
    }
    pub fn controller_state(&self, controller: ControllerId) -> hid::Result<ControllerState> {
        unimplemented!{}
    }
    pub fn controller_button_state(&self, controller: ControllerId, button: ControllerButton) -> hid::Result<ButtonState> {
        unimplemented!{}
    }
    pub fn controller_axis_state(&self, controller: ControllerId, axis: ControllerAxis) -> hid::Result<f64> {
        unimplemented!{}
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
    pub fn keyboard_key_state(&self, keyboard: KeyboardId, key: ScanCode) -> hid::Result<KeyState> {
        unimplemented!{}
    }
    pub fn key_name(&self, key: KeyCode) -> hid::Result<String> {
        unimplemented!{}
    }
    pub fn translate_scan_code(&self, keyboard: KeyboardId, scan_code: ScanCode) -> hid::Result<KeyCode> {
        unimplemented!{}
    }
    pub fn untranslate_key_code(&self, keyboard: KeyboardId, key_code: KeyCode) -> hid::Result<ScanCode> {
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
