//! Graphics tablets.

// + TODO Touch features ?
// + Q: Can we recognize pad buttons ? A: No and actually we don't care that much ??
// + Q: Can we recognize styli ? A: Yes, WinTab says that styli can be assigned a unique ID (introduced with Intuos tablets).
// + Q: Can we get the tablet's layout ? (answer: yes, use libwacom)
// For future extensions, see http://www.wacomeng.com/windows/docs/NotesForTabletAwarePCDevelopers.html

use context::Context;
use window::Window;
use os::{OsTabletPadButtonsState, OsTabletStylusButtonsState};
use super::{DeviceID, AxisInfo, ButtonState, Result};
use Vec2;

/// Tablet-specific information.
#[derive(Debug, Clone, PartialEq)]
pub struct TabletInfo {
    /// Information about the pressure axis.
    pub pressure_axis: AxisInfo,
    /// Information about the tilt axii.
    pub tilt_axis: Vec2<AxisInfo>,
    /// Information about the `physical_position` axis.
    pub physical_position_axis: AxisInfo,
}

/// Possible tool types for a tablet stylus.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum TabletStylusToolType {
    Pen, Eraser,
}

/// Possible kinds of tablet styli.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum TabletStylusKind {
    /// A regular stylus.
    Regular,
    /// A Wacom ArtPen.
    ArtPen,
    /// An airbrush.
    Airbrush,
    /// A 4D Mouse.
    FourDMouse,
    #[allow(missing_docs)]
    FiveButtonPuck,
}

/// A tablet pad button is a single platform-specific integer for now.
pub type TabletPadButton = i32;

/// Possible buttons for a tablet stylus.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum TabletStylusButton {
    /// The primary button is the one that is closest to the tip.
    Primary,
    /// The secondary button is the one that is right above the primary one along the stylus.
    Secondary,
    /// Another, unknown, button identifier.
    Other(i32),
}

/// Opaque container for the state of all of a tablet pad's buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TabletPadButtonsState(pub(crate) OsTabletPadButtonsState);

/// Opaque container for the state of all of a tablet stylus's buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TabletStylusButtonsState(pub(crate) OsTabletStylusButtonsState);

/// Snapshot of a tablet's state.
#[derive(Debug, Clone, PartialEq)]
pub struct TabletState {
    pub(crate) pad_buttons: TabletPadButtonsState,
    pub(crate) stylus_buttons: TabletStylusButtonsState,
    /// The root position of the cursor associated with this tablet.
    pub root_position: Vec2<f64>,
    /// The physical position of the stylus, expressed in terms of
    /// `TabletInfo::physical_position_axis`.
    pub physical_position: Vec2<f64>,
    /// The pressure of the stylus, expressed in terms of `TabletInfo::pressure_axis`.
    pub pressure: f64,
    /// The stylus's tilt, expressed in terms of `TabletInfo::tilt_axis`.
    pub tilt: Vec2<f64>,
    /// The current tool type.
    pub tool_type: TabletStylusToolType,
    /// The kind for the current stylus.
    pub stylus_kind: TabletStylusKind,
}

/// Snapshot of a tablet's state, relative to a window.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowTabletState {
    /// Other state not related to any window.
    pub global: TabletState,
    /// The position in window-space coordinates.
    pub position: Vec2<f64>,
}

impl TabletPadButtonsState {
    pub fn button(&self, button: TabletPadButton) -> Option<ButtonState> {
        self.0.button(button)
    }
}
impl TabletStylusButtonsState {
    pub fn button(&self, button: TabletStylusButton) -> Option<ButtonState> {
        self.0.button(button)
    }
}

impl TabletState {
    /// Gets the state of the given pad button.
    pub fn pad_button(&self, button: TabletPadButton) -> Option<ButtonState> {
        self.pad_buttons.button(button)
    }
    /// Gets the state of the given stylus button.
    pub fn stylus_button(&self, button: TabletStylusButton) -> Option<ButtonState> {
        self.stylus_buttons.button(button)
    }
}

impl Context {
    /// Fetches the current state of a tablet which ID is given.
    pub fn tablet_state(&self, tablet: DeviceID) -> Result<TabletState> {
        self.0.tablet_state(tablet)
    }
}

impl Window {
    /// Fetches the current state of a tablet which ID is given, relatively to this window.
    pub fn tablet_state(&self, tablet: DeviceID) -> Result<WindowTabletState> {
        self.0.tablet_state(tablet)
    }
}
