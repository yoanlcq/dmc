//! Graphics tablets.

// + TODO Touch features ?
// + Q: Can we recognize pad buttons ? A: No and actually we don't care that much ??
// + Q: Can we recognize styli ? A: Yes, WinTab says that styli can be assigned a unique ID (introduced with Intuos tablets).
// + Q: Can we get the tablet's layout ? (answer: yes, use libwacom)
// For future extensions, see http://www.wacomeng.com/windows/docs/NotesForTabletAwarePCDevelopers.html

use context::Context;
use window::Window;
use os::{OsTabletPadButtonsState, OsTabletStylusButtonsState, OsTabletInfo};
use super::{DeviceID, AxisInfo, ButtonState, Result};
use Vec2;

/// Tablet-specific information.
#[derive(Debug, Clone, PartialEq)]
pub struct TabletInfo(pub(crate) OsTabletInfo);

impl TabletInfo {
    /// Information about the pressure axis.
    pub fn pressure_axis(&self) -> &AxisInfo { self.0.pressure_axis() }
    /// Information about the tilt axii.
    pub fn tilt_axis(&self) -> Vec2<&AxisInfo> { self.0.tilt_axis() }
    /// Information about the `physical_position` axis.
    pub fn physical_position_axis(&self) -> &AxisInfo { self.0.physical_position_axis() }
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
    pub(crate) root_position: Vec2<f64>,
    /// The physical position of the stylus, expressed in terms of
    /// `TabletInfo::physical_position_axis`.
    pub(crate) physical_position: Vec2<f64>,
    /// The pressure of the stylus, expressed in terms of `TabletInfo::pressure_axis`.
    pub(crate) pressure: f64,
    /// The stylus's tilt, expressed in terms of `TabletInfo::tilt_axis`.
    pub(crate) tilt: Vec2<f64>,
    /// The current tool type.
    pub(crate) tool_type: TabletStylusToolType,
    /// The kind for the current stylus.
    pub(crate) stylus_kind: TabletStylusKind,
}

impl TabletState {
    /// The root position of the cursor associated with this tablet.
    pub fn root_position(&self) -> Vec2<f64> { self.root_position }
    /// The physical position of the stylus, expressed in terms of
    /// `TabletInfo::physical_position_axis`.
    pub fn physical_position(&self) -> Vec2<f64> { self.physical_position }
    /// The pressure of the stylus, expressed in terms of `TabletInfo::pressure_axis`.
    pub fn pressure(&self) -> f64 { self.pressure }
    /// The stylus's tilt, expressed in terms of `TabletInfo::tilt_axis`.
    pub fn tilt(&self) -> Vec2<f64> { self.tilt }
    /// The current tool type.
    pub fn tool_type(&self) -> TabletStylusToolType { self.tool_type }
    /// The kind for the current stylus.
    pub fn stylus_kind(&self) -> TabletStylusKind { self.stylus_kind }
}

/// Snapshot of a tablet's state, relative to a window.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowTabletState {
    /// Other state not related to any window.
    pub(crate) global: TabletState,
    /// The position in window-space coordinates.
    pub(crate) position: Option<Vec2<f64>>,
}

impl WindowTabletState {
    /// Other state not related to any window.
    pub fn global(&self) -> &TabletState { &self.global }
    /// The position in window-space coordinates, if the cursor is within the window.
    pub fn position(&self) -> Option<Vec2<f64>> { self.position }
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
