//! Mice.

use context::Context;
use window::Window;
use os::{OsMouseID, OsMouseButtonsState, OsDeviceID};
use super::{ButtonState, Result};
use Vec2;

/// A device ID type for mice.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct MouseID(pub(crate) OsMouseID);
impl OsDeviceID for MouseID {}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MouseButtonsState(pub(crate) OsMouseButtonsState);

/// A snapshot of the mouse's global state.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseState {
    pub(crate) buttons: MouseButtonsState,
    /// The position relative to the "root window".
    pub root_position: Vec2<f64>,
}

/// A snapshot of the mouse's state, relatively to a window.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowMouseState {
    /// The global part of the state.
    pub global: MouseState,
    /// The position, in window coordinates.
    pub position: Option<Vec2<f64>>,
}

impl MouseButtonsState {
    pub fn button(&self, button: MouseButton) -> Option<ButtonState> {
        self.0.button(button)
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
    pub fn mice(&self) -> Result<Vec<MouseID>> {
        self.0.mice()
    }
    /// Gets the ID for the main mouse, if any.
    pub fn main_mouse(&self) -> Result<MouseID> {
        self.0.main_mouse()
    }
    /// Captures the current state of the mouse which ID is given.
    pub fn mouse_state(&self, mouse: MouseID) -> Result<MouseState> {
        self.0.mouse_state(mouse)
    }
}

impl Window {
    /// Captures the current state of the mouse which ID is given, relatively to this window.
    pub fn mouse_state(&self, mouse: MouseID) -> Result<WindowMouseState> {
        self.0.mouse_state(mouse)
    }
}
