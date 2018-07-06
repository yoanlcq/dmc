//! Mice.

use context::Context;
use window::Window;
use os::OsMouseButtonsState;
use super::{DeviceID, ButtonState, Result};
use Vec2;

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

/// On Windows, the XBUTTON1 mouse button is the "Back" variant.
pub const XBUTTON1: MouseButton = MouseButton::Back;
/// On Windows, the XBUTTON2 mouse button is the "Forward" variant.
pub const XBUTTON2: MouseButton = MouseButton::Forward;

/// An opaque container for the current state a mouse's buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MouseButtonsState(pub(crate) OsMouseButtonsState);

/// There's nothing in here, for now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseInfo;

/// A snapshot of the mouse's global state.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseState {
    pub(crate) buttons: MouseButtonsState,
    /// The position relative to the "root window".
    pub(crate) root_position: Vec2<f64>,
}

/// A snapshot of the mouse's state, relatively to a window.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowMouseState {
    /// The global part of the state.
    pub(crate) global: MouseState,
    /// The position, in window coordinates.
    pub(crate) position: Option<Vec2<f64>>,
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
    /// Gets the position of the mouse in virtual desktop coordinates.
    pub fn root_position(&self) -> Vec2<f64> {
        self.root_position
    }
}

impl WindowMouseState {
    /// Gets the `MouseState`, i.e state that is not specific to a window.
    pub fn global(&self) -> &MouseState {
        &self.global
    }
    /// Gets the position of the mouse in window coordinates, or `None` if the mouse is not within
    /// the window.
    pub fn position(&self) -> Option<Vec2<f64>> {
        self.position
    }
}

impl Context {
    /// Gets the ID for the main mouse, if any.
    pub fn main_mouse(&self) -> Result<DeviceID> {
        self.0.main_mouse()
    }
    /// Captures the current state of the mouse which ID is given.
    pub fn mouse_state(&self, mouse: DeviceID) -> Result<MouseState> {
        self.0.mouse_state(mouse)
    }
}

impl Window {
    /// Captures the current state of the mouse which ID is given, relatively to this window.
    pub fn mouse_state(&self, mouse: DeviceID) -> Result<WindowMouseState> {
        self.0.mouse_state(mouse)
    }
}
