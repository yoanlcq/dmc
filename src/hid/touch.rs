//! Touch devices (touchpads or touch-screens)
//!
//! This module is very incomplete.

use context::Context;
use os::{OsTouchId, OsDeviceId};
use super::{AxisInfo, Result};

/// A device ID type for touch devices.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TouchId(pub(crate) OsTouchId);
impl OsDeviceId for TouchId {}

/// Touch-device-specific information.
pub struct TouchInfo {
    /// The `AxisInfo` for the pressure axis.
    pub pressure_axis: AxisInfo,
}

impl Context {
    /// Lists all connected touch devices.
    pub fn touch_devices(&self) -> Result<Vec<TouchId>> {
        self.0.touch_devices()
    }
    /// Fetches the `TouchInfo` associated to the given device ID.
    pub fn touch_info(&self, touch: TouchId) -> Result<TouchInfo> {
        self.0.touch_info(touch)
    }
}

