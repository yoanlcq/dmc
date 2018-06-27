use device::{
    self,
    DeviceID, ButtonState,
    MouseButton, MouseState, WindowMouseState,
};
use os::{OsContext, OsWindow, OsDeviceID};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsMouseButtonsState;

impl OsMouseButtonsState {
    pub fn button(&self, button: MouseButton) -> Option<ButtonState> {
        unimplemented!()
    }
}

impl OsContext {
    pub fn main_mouse(&self) -> device::Result<DeviceID> {
        Ok(DeviceID(OsDeviceID::MainMouse))
    }
    pub fn mouse_state(&self, mouse: DeviceID) -> device::Result<MouseState> {
        unimplemented!()
    }
}

impl OsWindow {
    pub fn mouse_state(&self, mouse: DeviceID) -> device::Result<WindowMouseState> {
        unimplemented!()
    }
}

