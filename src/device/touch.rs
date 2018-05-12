//! Touch devices (touchpads or touch-screens)
//!
//! This module is very incomplete.

use super::AxisInfo;

/// Touch-device-specific information.
#[derive(Debug, Clone, PartialEq)]
pub struct TouchInfo {
    /// The `AxisInfo` for the pressure axis.
    pub(crate) pressure_axis: AxisInfo,
}

impl TouchInfo {
    pub fn pressure_axis(&self) -> &AxisInfo { &self.pressure_axis }
}
