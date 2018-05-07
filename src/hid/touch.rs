//! Touch devices (touchpads or touch-screens)
//!
//! This module is very incomplete.

use context::Context;
use super::{HidID, AxisInfo, Result};

/// Touch-device-specific information.
#[derive(Debug, Clone, PartialEq)]
pub struct TouchInfo {
    /// The `AxisInfo` for the pressure axis.
    pub pressure_axis: AxisInfo,
}

impl Context {
    /// Lists all connected touch devices.
    pub fn touch_devices(&self) -> Result<Vec<HidID>> {
        self.0.touch_devices()
    }
}

