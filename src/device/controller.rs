//! Controllers (Gamepads, Joysticks, Steering wheels, etc).
//!
//! # F.A.Q
//!
//! ## What is meant by "Controller"?
//!
//! A catch-all term for "game input devices". A controller can be classified as one
//! or more of the following :
//!
//! - Gamepads, such as Xbox gamepads and DualShock gamepads;
//! - Steering wheels, for racing games;
//! - Joysticks, as in "actual, single, large joystick devices";
//!
//! On Linux, `udev` reports all of these with the `ID_INPUT_JOYSTICK` property set to `1`,
//! (even though a gamepad is not a joystick so to speak), which is how we know we can
//! attempt to open the device file in order to read events (or write force-feedback effects)
//! ourselves (because other devices require super-user privileges to open their device file).
//!
//! In any case, most (if not all) OSes do not treat controllers in the same way as mice, keyboards, etc,
//! because they are mostly specific to games, they (normally) don't control the desktop, etc.  
//! So, the APIs used to deal with them is usually separate from the "more commonly used" system APIs.
//!
//!
//! ## Why do Y axes go down?
//!
//! Because this appears to be the most widespread standard for gamepad input, but I might be wrong.  
//!
//! I personnaly prefer "positive Y goes up", but sticking to the most widespread convention reduces
//! maintenance efforts and overall likeliness of bugs.
//!
//! Hopefully, everybody seems to agree that positive X goes right!
//!
//! ### In favor of "positive Y goes down" :
//!
//! - [Linux Gamepad Specification](https://www.kernel.org/doc/html/v4.16/input/gamepad.html):
//!   "for ABS values negative is left/up, positive is right/down".
//! - [W3 Gamepad working draft](https://www.w3.org/TR/gamepad):
//!   "As appropriate, -1.0 SHOULD correspond to "up" or "left", and 1.0 SHOULD correspond to "down" or "right"".
//!
//! ### In favor of "positive Y goes up" :
//!
//! - [`XINPUT_GAMEPAD` structure](https://msdn.microsoft.com/en-us/library/windows/desktop/microsoft.directx_sdk.reference.xinput_gamepad(v=vs.85).aspx):
//!   "Negative values signify down or to the left. Positive values signify up or to the right".
//!
//!
//! ## Why is the D-pad both 4 buttons and 2 axes?
//!
//! A given gamepad either supports:
//!
//! - `DpadDown`, `DpadLeft`, `DpadUp` and `DpadRight` **buttons**, OR
//! - `DpadX` and `DpadY` **axes**, OR
//! - Both of them, OR
//! - None of them.
//!
//! Your application should handle all of these cases properly; However, this looks annoying
//! to deal with. Couldn't this crate deal with these details by itself and provide
//! a single, unified way to deal with D-pads?   
//! No, and here's why :
//!
//! Quoting the [Linux Gamepad Specification](https://www.kernel.org/doc/html/v4.16/input/gamepad.html):
//! 
//! > Every gamepad provides a D-Pad with four directions: Up, Down, Left, Right.  
//! > Some of these are available as digital buttons, some as analog buttons. Some may even report both.  
//! > The kernel does not convert between these so applications should support both and choose what 
//! > is more appropriate if both are reported.
//! >
//! > - Digital buttons are reported as: `BTN_DPAD_*`
//! > - Analog buttons are reported as: `ABS_HAT0X and ABS_HAT0Y`
//!
//! Trying to cope with this would be lying, unreliable, and annoying to do anyway.  
//!
//! As usual, you may write your own facilities for converting D-pad-related events into the
//! representation that is most convenient for you.
//!
//!
//! ## How can I reliably identify a controller model? Why doesn't this crate do this for me?
//!
//! Identifying controller models may be important to some gameplay elements, such as displaying
//! butons on-screen as they look on the controller.
//!
//! Use the USB vendor ID and product ID. In all cases, **avoid** using the device's name (even
//! though it may just do the job).
//! 
//! The task of identifying devices from USB IDs is best left to another library, but if you know
//! in advance the subset of devices you want to support, just look up their USB IDs in some public
//! online database, and write a function that uses these.

use context::Context;
use os::{self, OsControllerState, OsControllerInfo};
use super::{DeviceID, ButtonState, AxisInfo, Result};

/// Opaque container for a snapshot of a controller's full state.
#[derive(Debug, Clone, PartialEq)]
pub struct ControllerState(pub(crate) OsControllerState);

/// Information for a controller.
#[derive(Debug, Clone, PartialEq)]
pub struct ControllerInfo(pub(crate) OsControllerInfo);

/// A rumble effect description.
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct VibrationState {
    /// Magnitude for the strong, high-amplitude, high-frequency, right motor.
    /// 0 signifies no motor use, and 65353 signifies 100% motor use.
    pub strong_magnitude: u16,
    /// Magnitude for the weak, low-amplitude, low-frequency, left motor.
    /// 0 signifies no motor use, and 65353 signifies 100% motor use.
    pub weak_magnitude: u16,
}

impl VibrationState {
    /// The maximum value for a vibration state. This is a convenience for not writing
    /// `::std::u16::MAX`.
    pub const MAX: Self = Self {
        strong_magnitude: ::std::u16::MAX,
        weak_magnitude: ::std::u16::MAX,
    };
    /// Does this correspond to zero vibration?
    pub fn is_zero(&self) -> bool {
        self.strong_magnitude == 0 && self.weak_magnitude == 0
    }
}


/// A known controller button.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum ControllerButton {
    /// (Gamepad, digital D-pad) The "up" button on the D-pad.
    DpadUp,
    /// (Gamepad, digital D-pad) The "down" button on the D-pad.
    DpadDown,
    /// (Gamepad, digital D-pad) The "left" button on the D-pad.
    DpadLeft,
    /// (Gamepad, digital D-pad) The "right" button on the D-pad.
    DpadRight,
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

    /// (Unknown) A number button. Values start at 0, and there's usually a maximum of 10.
    ///
    /// This is not some fallback value for any kind of button; They are defined
    /// for buttons which name is literally a number.
    Num(u32),

    /// An other, unknown, backend-specific button.
    Other(i32),
}

impl ControllerButton {
    /// Convenience alias for the up-most button in a gamepad's right cluster. This is always `Y`.
    pub const NORTH: Self = ControllerButton::Y;
    /// Convenience alias for the down-most button in a gamepad's right cluster. This is always `A`.
    pub const SOUTH: Self = ControllerButton::A;
    /// Convenience alias for the left-most button in a gamepad's right cluster. This is always `X`.
    pub const WEST: Self = ControllerButton::X;
    /// Convenience alias for the right-most button in a gamepad's right cluster. This is always `B`.
    pub const EAST: Self = ControllerButton::B;
    /// Platform-specific hard limit on the number of `Thumb` buttons. `None` means that no limit is known.
    pub const MAX_THUMB: Option<u32> = os::device_consts::MAX_THUMB_BUTTONS;
    /// Platform-specific hard limit on the number of `Top` buttons. `None` means that no limit is known.
    pub const MAX_TOP: Option<u32> = os::device_consts::MAX_TOP_BUTTONS;
    /// Platform-specific hard limit on the number of `Base` buttons. `None` means that no limit is known.
    pub const MAX_BASE: Option<u32> = os::device_consts::MAX_BASE_BUTTONS;
    /// Platform-specific hard limit on the number of `Num` buttons. `None` means that no limit is known.
    pub const MAX_NUM: Option<u32> = os::device_consts::MAX_NUM_BUTTONS;
}

/// A known controller axis.
///
/// All of these axies are absolute; Relative axes are normally only relevant for e.g mice or
/// scroll wheels.
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
    /// (Gamepads, analog D-pad) The D-pad's horizontal position, increasing rightwards.
    DpadX,
    /// (Gamepads, analog D-pad) The D-pad's vertical position, increasing downwards.
    DpadY,

    /// (Joysticks) The main joystick's X position (TODO: which direction is it actually?).
    JoystickX,
    /// (Joysticks) The main joystick's Y position (TODO: which direction is it actually?).
    JoystickY,
    /// (Joysticks) The main joystick's Z position (TODO: which direction is it actually?).
    JoystickZ,
    /// (Joysticks) The main joystick's X rotation.
    JoystickRotationX,
    /// (Joysticks) The main joystick's Y rotation.
    JoystickRotationY,
    /// (Joysticks) The main joystick's Z rotation.
    JoystickRotationZ,

    /// (Joysticks) A hat's X position, increasing rightwards. Hats are numbered from 0 to (usually) 3, inclusive.
    HatX(u32),
    /// (Joysticks) A hat's Y position, increasing downwards. Hats are numbered from 0 to (usually) 3, inclusive.
    HatY(u32),

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

impl ControllerAxis {
    /// Platform-specific hard limit on the number of hats (each hat offers a `HatX` and `HatY` axis). `None` means that no limit is known.
    pub const MAX_HAT: Option<u32> = os::device_consts::MAX_HAT_AXES;
}


impl ControllerInfo {
    /// Is this controller a gamepad?
    pub fn is_a_gamepad(&self) -> bool {
        self.0.is_a_gamepad()
    }
    /// Is this controller reported as a joystick?
    pub fn is_a_joystick(&self) -> bool {
        self.0.is_a_joystick()
    }
    /// Is this controller reported as a steering wheel?
    pub fn is_a_steering_wheel(&self) -> bool {
        self.0.is_a_steering_wheel()
    }
    /// Does this controller support rumble effects?
    pub fn supports_rumble(&self) -> bool {
        self.0.supports_rumble()
    }
    /// Does this controller have the given button?
    pub fn has_button(&self, button: ControllerButton) -> bool {
        self.0.has_button(button)
    }
    /// Does this controller have the given axis?
    pub fn has_axis(&self, axis: ControllerAxis) -> bool {
        self.0.has_axis(axis)
    }
    /// Gets the `AxisInfo` for the given controller axis if the controller has it.
    pub fn axis(&self, axis: ControllerAxis) -> Option<&AxisInfo> {
        self.0.axis(axis)
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

impl Context {
    /// Gets a snapshot of a controller's current state, which ID is given.
    pub fn controller_state(&self, controller: DeviceID) -> Result<ControllerState> {
        self.0.controller_state(controller)
    }
    /// Gets the current state of a button for the controller which ID is given.
    pub fn controller_button_state(&self, controller: DeviceID, button: ControllerButton) -> Result<ButtonState> {
        self.0.controller_button_state(controller, button)
    }
    /// Gets the current state of an axis for the controller which ID is given.
    pub fn controller_axis_state(&self, controller: DeviceID, axis: ControllerAxis) -> Result<f64> {
        self.0.controller_axis_state(controller, axis)
    }
    /// Sets the vibration state for the controller which ID is given, if the device supports it.
    ///
    /// To stop vibrations, just set relevant members of `VibrationState` to zero.
    ///
    /// **N.B**: The vibration state **may or may not** be reset as the device is dropped. For
    /// instance, it may be the case that you want to write a utility application that sets the
    /// vibration state for a controller and then exits.  
    /// If you want to be extra sure, reset it yourself when your application exits.
    pub fn controller_set_vibration(&self, controller: DeviceID, vibration: &VibrationState) -> Result<()> {
        self.0.controller_set_vibration(controller, vibration)
    }
}

