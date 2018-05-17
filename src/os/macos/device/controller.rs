use device::{
    self,
    DeviceID, AxisInfo, ButtonState,
    ControllerAxis, ControllerButton, ControllerState, VibrationState,
};
use os::OsContext;

#[derive(Debug, Clone, PartialEq)]
pub struct OsControllerState;
#[derive(Debug, Clone, PartialEq)]
pub struct OsControllerInfo;

impl OsControllerInfo {
    pub fn is_a_gamepad(&self) -> bool {
        unimplemented!()
    }
    pub fn is_a_joystick(&self) -> bool {
        unimplemented!()
    }
    pub fn is_a_steering_wheel(&self) -> bool {
        unimplemented!()
    }
    pub fn supports_rumble(&self) -> bool {
        unimplemented!()
    }
    pub fn has_button(&self, button: ControllerButton) -> bool {
        unimplemented!()
    }
    pub fn has_axis(&self, axis: ControllerAxis) -> bool {
        unimplemented!()
    }
    pub fn axis(&self, axis: ControllerAxis) -> Option<&AxisInfo> {
        unimplemented!()
    }
}

impl OsControllerState {
    pub fn button(&self, button: ControllerButton) -> Option<ButtonState> {
        unimplemented!()
    }
    pub fn axis(&self, axis: ControllerAxis) -> Option<f64> {
        unimplemented!()
    }
}

impl OsContext {
    pub fn controller_state(&self, controller: DeviceID) -> device::Result<ControllerState> {
        unimplemented!()
    }
    pub fn controller_button_state(&self, controller: DeviceID, button: ControllerButton) -> device::Result<ButtonState> {
        unimplemented!()
    }
    pub fn controller_axis_state(&self, controller: DeviceID, axis: ControllerAxis) -> device::Result<f64> {
        unimplemented!()
    }
    pub fn controller_set_vibration(&self, controller: DeviceID, vibration: &VibrationState) -> device::Result<()> {
        unimplemented!()
    }
}

