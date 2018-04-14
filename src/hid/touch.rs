//! Touch devices (touchpads or touch-screens)

use context::Context;
use os::OsTouchId;
use super::{DeviceId, AxisInfo, Result};

/// A device ID type for touch devices.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TouchId(pub(crate) OsTouchId);
impl DeviceId for TouchId {}

/// Touch-device-specific information.
pub struct TouchInfo {
    /// The `AxisInfo` for the pressure axis.
    pub pressure_axis: AxisInfo,
}

impl Context {
    /// Lists all connected touch devices.
    pub fn touch_devices(&self) -> Result<Vec<TouchId>> {
        unimplemented!{}
    }
    /// Fetches the `TouchInfo` associated to the given device ID.
    pub fn touch_info(&self, touch: TouchId) -> Result<TouchInfo> {
        unimplemented!{}
    }
}

