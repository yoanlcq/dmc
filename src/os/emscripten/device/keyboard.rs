use device::{
    self,
    DeviceID, KeyState,
    KeyboardState, Keysym, Keycode,
};
use os::OsContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsKeyboardState;
pub type OsKeycode = ();
pub type OsKeysym = ();

impl OsContext {
    pub fn main_keyboard(&self) -> device::Result<DeviceID> {
        unimplemented!()
    }
    pub fn keyboard_state(&self, keyboard: DeviceID) -> device::Result<KeyboardState> {
        unimplemented!()
    }
    pub fn keyboard_keycode_state(&self, keyboard: DeviceID, keycode: Keycode) -> device::Result<KeyState> {
        unimplemented!()
    }
    pub fn keyboard_keysym_state(&self, keyboard: DeviceID, keysym: Keysym) -> device::Result<KeyState> {
        unimplemented!()
    }
    pub fn keysym_name(&self, keysym: Keysym) -> device::Result<String> {
        unimplemented!()
    }
    pub fn keysym_from_keycode(&self, keyboard: DeviceID, keycode: Keycode) -> device::Result<Keysym> {
        unimplemented!()
    }
    pub fn keycode_from_keysym(&self, keyboard: DeviceID, keysym: Keysym) -> device::Result<Keycode> {
        unimplemented!()
    }
}

impl OsKeyboardState {
    pub fn keycode(&self, keycode: Keycode) -> Option<KeyState> {
        unimplemented!()
    }
    pub fn keysym(&self, keysym: Keysym) -> Option<KeyState> {
        unimplemented!()
    }
}
