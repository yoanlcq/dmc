//! Controllers (Gamepads, Joysticks, Steering wheels, etc).
//
// TODO: Haptic features for those three.
//
// Gamepad
// Joystick
// Steering Wheel
// rationale : udev treats all these as ID_INPUT_JOYSTICK.

use context::Context;
use os::{OsControllerID, OsControllerState, OsControllerInfo, OsDeviceID};
use super::{ButtonState, AxisInfo, Result};

/// A device ID type for controllers.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ControllerID(pub(crate) OsControllerID);
impl OsDeviceID for ControllerID {}

/// Opaque container for a snapshot of a controller's full state.
#[derive(Debug, Clone, PartialEq)]
pub struct ControllerState(pub(crate) OsControllerState);

/// Information for a controller.
#[derive(Debug, Clone, PartialEq)]
pub struct ControllerInfo {
    pub(crate) internal: OsControllerInfo,
    /// The kind of controller.
    pub kind: ControllerKind,
}

/// Possible kinds of controllers.
///
/// There might be more in the future as this crate grows.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum ControllerKind {
    /// A gamepad.
    Gamepad(GamepadModel),
    /// A single joystick.
    Joystick,
    /// A steering wheel.
    SteeringWheel,
}

/// Well-known gamepad models.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum GamepadModel {
    Xbox360,
    XboxOne,
    Generic,
    // DualShock...
}

impl ControllerInfo {
    /// Does this controller have the given button?
    pub fn has_button(&self, button: ControllerButton) -> bool {
        self.internal.has_button(button)
    }
    /// Does this controller have the given axis?
    pub fn has_axis(&self, axis: ControllerAxis) -> bool {
        self.internal.has_axis(axis)
    }
    /// Gets the `AxisInfo` for the given controller axis if the controller has it.
    pub fn axis(&self, axis: ControllerAxis) -> Option<AxisInfo> {
        self.internal.axis(axis)
    }
}

impl Context {
    /// Lists all connected controller devices.
    pub fn controllers(&self) -> Result<Vec<ControllerID>> {
        self.0.controllers()
    }
    /// Fetches the `ControllerInfo` for the controller which ID is given.
    pub fn controller_info(&self, controller: ControllerID) -> Result<ControllerInfo> {
        self.0.controller_info(controller)
    }
    /// Gets a snapshot of a controller's current state, which ID is given.
    pub fn controller_state(&self, controller: ControllerID) -> Result<ControllerState> {
        self.0.controller_state(controller)
    }
    /// Gets the current state of a button for the controller which ID is given.
    pub fn controller_button_state(&self, controller: ControllerID, button: ControllerButton) -> Result<ButtonState> {
        self.0.controller_button_state(controller, button)
    }
    /// Gets the current state of an axis for the controller which ID is given.
    pub fn controller_axis_state(&self, controller: ControllerID, axis: ControllerAxis) -> Result<f64> {
        self.0.controller_axis_state(controller, axis)
    }
}

impl ControllerState {
    /// Gets the state of the given button.
    pub fn button(&self, button: ControllerButton) -> Option<ButtonState> {
        self.0.button(button)
    }
    /// Gets the state of the given axis.
    pub fn axis(&self, axis: ControllerAxis) -> Option<f64> {
        self.0.axis(axis)
    }
}


/// A known controller button.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum ControllerButton {
    /// (Gamepads) The X button, as for an Xbox 360 controller.
    X,
    /// (Gamepads) The Y button, as for an Xbox 360 controller.
    Y, 
    /// (Gamepads) The Z button; If a controller has it, it is normally
    /// the one that follows after the X and Y buttons.
    Z, 
    /// (Gamepads) The A button, as for an Xbox 360 controller.
    A, 
    /// (Gamepads) The B button, as for an Xbox 360 controller.
    B, 
    /// (Gamepads) The C button; If a controller has it, it is normally
    /// the one that follows after the A and B buttons.
    C,
    /// (Gamepads) The first left shoulder, as the L1 button on a DualShock controller.
    LShoulder,
    /// (Gamepads) The second left shoulder, as the L2 button on a DualShock controller.
    LShoulder2, 
    /// (Gamepads) The first right shoulder, as the R1 button on a DualShock controller.
    RShoulder,
    /// (Gamepads) The second right shoulder, as the R2 button on a DualShock controller.
    RShoulder2,
    /// (Gamepads) Pressing (clicking) the left stick.
    LStickClick,
    /// (Gamepads) Pressing (clicking) the right stick.
    RStickClick,
    /// (Gamepads) The leftmost button in the gamepad's center, as the "Select" button on a DualShock controller.
    Select,
    /// (Gamepads) The rightmost button in the gamepad's center, as the "Start" button on a DualShock controller.
    Start,
    /// (Gamepads) The button at the gamepad's center, as the "Mode" button on a DualShock controller or the glowing "X" at the center of a Xbox gamepad.
    Mode,

    /// (Steering wheels) Gear down.
    GearDown,
    /// (Steering wheels) Gear up.
    GearUp,

    /// (Joysticks) The trigger button.
    Trigger,
    /// (Joysticks) The pinkie button.
    Pinkie,
    /// (Joysticks) I have no idea what this is but Linux exposes it.
    Dead,
    /// (Joysticks) A thumb button. Indices start at 0, and there's usually a maximum of 2.
    Thumb(u32),
    /// (Joysticks) A top button. Indices start at 0, and there's usually a maximum of 2.
    Top(u32),
    /// (Joysticks) A base button. Indices start at 0, and there's usually a maximum of 6.
    Base(u32),

    /// An other, unknown, backend-specific button.
    Other(i32),
}

/// A known controller axis.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum ControllerAxis {
    /// (Gamepads) The left stick's horizontal position, increasing rightwards.
    LX,
    /// (Gamepads) The left stick's vertical position, increasing downwards.
    LY,
    /// (Gamepads) The right stick's horizontal position, increasing rightwards.
    RX,
    /// (Gamepads) The right stick's vertical position, increasing downwards.
    RY,
    /// (Gamepads) The D-Pad's horizontal position, increasing rightwards.
    DpadX,
    /// (Gamepads) The D-Pad's vertical position, increasing downwards.
    DpadY,

    /// (Joysticks) The main joystick's X position (TODO: which direction is it actually?).
    X,
    /// (Joysticks) The main joystick's Y position (TODO: which direction is it actually?).
    Y,
    /// (Joysticks) The main joystick's Z position (TODO: which direction is it actually?).
    Z,

    /// (Gamepads) The left trigger axis, as for Xbox controllers.
    LTrigger,
    /// (Gamepads) The right trigger axis, as for Xbox controllers.
    RTrigger,

    /// (Steering wheels) The throttle pedal.
    Throttle,
    /// (Steering wheels) The rudder.
    Rudder,
    /// (Steering wheels) The wheel's rotation.
    Wheel,
    /// (Steering wheels) The gas pedal.
    Gas,
    /// (Steering wheels) The break pedal.
    Brake,

    /// An other, unknown, backend-specific button.
    Other(i32),
}
