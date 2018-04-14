//! Mice.

use context::Context;
use window::Window;
use os::{OsMouseId, OsMouseButtonsState};
use super::{DeviceId, ButtonState, Result};
use Vec2;

/// A device ID type for mice.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MouseId(pub(crate) OsMouseId);
impl DeviceId for MouseId {}

/// Known mouse buttons.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum MouseButton {
    /// The left mouse button.
    Left,
    /// The middle mouse button (clicking on the wheel).
    Middle,
    /// The right mouse button.
    Right,
    /// The side button, whatever it means to the platform, if any.
    Side,
    /// The "task" button, whatever it means to the platform, if any.
    Task,
    /// The "forward navigation" button, if any.
    Forward,
    /// The "backwards navigation" button, if any.
    Back,
    /// An other, unknown, platform-specific button.
    Other(i32),
}

/// An opaque container for the current state a mouse's buttons.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(crate) struct MouseButtonsState(pub(crate) OsMouseButtonsState);

/// A snapshot of the mouse's global state.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MouseState {
    pub(crate) buttons: MouseButtonsState,
    /// The position relative to the "root window".
    pub root_position: Vec2<f64>,
}

/// A snapshot of the mouse's state, relatively to a window.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct WindowMouseState {
    /// The global part of the state.
    pub global: MouseState,
    /// The position, in window coordinates.
    pub position: Option<Vec2<f64>>,
}

impl MouseButtonsState {
    pub fn button(&self, button: MouseButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}

impl MouseState {
    /// Gets the state of the given button for this mouse.
    pub fn button(&self, button: MouseButton) -> Option<ButtonState> {
        self.buttons.button(button)
    }
}


impl Context {
    /// Lists all currently connected mouse devices.
    pub fn mice(&self) -> Result<Vec<MouseId>> {
        unimplemented!{}
    }
    /// Gets the ID for the main mouse, if any.
    pub fn main_mouse(&self) -> Result<MouseId> {
        unimplemented!{}
    }
    /// Captures the current state of the mouse which ID is given.
    pub fn mouse_state(&self, mouse: MouseId) -> Result<MouseState> {
        unimplemented!{}
    }
    /// Attempts to set the mouse's position, relative to the "root window".
    pub fn set_mouse_root_position(&self, mouse: MouseId, root_position: Vec2<f64>) -> Result<()> {
        unimplemented!{}
    }
}

impl Window {
    /// Captures the current state of the mouse which ID is given, relatively to this window.
    pub fn mouse_state(&self, mouse: MouseId) -> Result<WindowMouseState> {
        unimplemented!{}
    }
}
