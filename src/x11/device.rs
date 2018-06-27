use std::slice;
use std::os::raw::c_int;
use super::x11::xinput2 as xi2;
use super::x11::xlib as x;
use super::X11SharedContext;
use device::{
    self,
    DeviceID, DeviceInfo, ButtonState, UsbIDs, Bus, AxisInfo,
    ControllerButton, ControllerAxis, ControllerState, ControllerInfo,
    VibrationState,
    KeyboardInfo, KeyState, KeyboardState, Keysym, Keycode,
    MouseInfo, MouseState, MouseButton,
    TabletInfo, TabletState, TabletPadButton, TabletStylusButton,
    TouchInfo,
};
use Vec2;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum X11DeviceID {
    CoreKeyboard,
    CorePointer,
    XISlave(c_int),
}

impl X11DeviceID {
    pub fn xi_device_id(&self) -> device::Result<c_int> {
        match *self {
            X11DeviceID::XISlave(x) => Ok(x),
            _ => device::failed("This device ID is not a XI device ID"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11TabletInfo;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11KeyboardState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11MouseButtonsState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11TabletPadButtonsState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11TabletStylusButtonsState;

impl X11TabletInfo {
    pub fn pressure_axis(&self) -> &AxisInfo { unimplemented!{} }
    pub fn tilt_axis(&self) -> Vec2<&AxisInfo> { unimplemented!{} }
    pub fn physical_position_axis(&self) -> &AxisInfo { unimplemented!{} }
}


impl X11KeyboardState {
    pub fn keycode(&self, key: Keycode) -> Option<KeyState> {
        unimplemented!{}
    }
    pub fn keysym(&self, key: Keysym) -> Option<KeyState> {
        unimplemented!{}
    }
}
impl X11MouseButtonsState {
    pub fn button(&self, button: MouseButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}
impl X11TabletPadButtonsState {
    pub fn button(&self, button: TabletPadButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}
impl X11TabletStylusButtonsState {
    pub fn button(&self, button: TabletStylusButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}

impl X11SharedContext {
    pub fn keyboard_state(&self, keyboard: X11DeviceID) -> device::Result<KeyboardState> {
        let x_display = self.lock_x_display();
        /*
        // Quoting the man page:
        // Byte N (from 0) contains the bits for keys 8N to 8N + 7 with the least significant bit in the byte representing key 8N.
        let mut key_bits: [u8; 32] = [0; 32];
        let _ = xlib_error::sync_catch(*x_display, || unsafe {
            x::XQueryKeymap(*x_display, key_bits.as_mut_ptr() as _)
        })?;
        unimplemented!{} // FIXME: We're completely ignoring the keyboard ID :(
        */

        let dev_infos = {
            let mut nb_infos = 0;
            let dev_infos = unsafe {
                xi2::XIQueryDevice(*x_display, unimplemented!{}, &mut nb_infos)
            };
            if nb_infos == 0 {
                return device::failed("XIQueryDevice() returned zero device info");
            }
            if dev_infos.is_null() { // FIXME: Can it though?
                return device::failed("XIQueryDevice() returned NULL");
            }
            unsafe {
                slice::from_raw_parts(dev_infos, nb_infos as _)
            }
        };

        unimplemented!{};

        unsafe {
            xi2::XIFreeDeviceInfo(dev_infos.as_ptr() as *mut _);
        }
        unimplemented!{}
    }
    pub fn keyboard_keycode_state(&self, keyboard: X11DeviceID, keycode: Keycode) -> device::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keyboard_keysym_state(&self, keyboard: X11DeviceID, keysym: Keysym) -> device::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keysym_name(&self, keysym: Keysym) -> device::Result<String> {
        unimplemented!{}
    }
    pub fn keysym_from_keycode(&self, keyboard: X11DeviceID, keycode: Keycode) -> device::Result<Keysym> {
        unimplemented!{}
    }
    pub fn keycode_from_keysym(&self, keyboard: X11DeviceID, keysym: Keysym) -> device::Result<Keycode> {
        unimplemented!{}
    }
    pub fn mouse_state(&self, mouse: X11DeviceID) -> device::Result<MouseState> {
        unimplemented!{}
    }
    pub fn tablet_state(&self, tablet: X11DeviceID) -> device::Result<TabletState> {
        unimplemented!{}
    }
}
