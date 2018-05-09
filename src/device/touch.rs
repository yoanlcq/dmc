//! Touch devices (touchpads or touch-screens)
//!
//! This module is very incomplete.

use context::Context;
use super::{DeviceID, AxisInfo, Result};

/// Touch-device-specific information.
#[derive(Debug, Clone, PartialEq)]
pub struct TouchInfo {
    /// The `AxisInfo` for the pressure axis.
    pub pressure_axis: AxisInfo,
}

