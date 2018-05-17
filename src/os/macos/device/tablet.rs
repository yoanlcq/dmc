use Vec2;
use device::{
    self,
    DeviceID, AxisInfo, ButtonState,
    TabletState, TabletPadButton, TabletStylusButton, WindowTabletState,
};
use os::{OsContext, OsWindow};

#[derive(Debug, Clone, PartialEq)]
pub struct OsTabletInfo;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsTabletPadButtonsState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsTabletStylusButtonsState;

impl OsTabletInfo {
    pub fn pressure_axis(&self) -> &AxisInfo { unimplemented!() }
    pub fn tilt_axis(&self) -> Vec2<&AxisInfo> { unimplemented!() }
    pub fn physical_position_axis(&self) -> &AxisInfo { unimplemented!() }
}

impl OsTabletPadButtonsState {
    pub fn button(&self, button: TabletPadButton) -> Option<ButtonState> {
        unimplemented!()
    }
}
impl OsTabletStylusButtonsState {
    pub fn button(&self, button: TabletStylusButton) -> Option<ButtonState> {
        unimplemented!()
    }
}

impl OsContext {
    pub fn tablet_state(&self, tablet: DeviceID) -> device::Result<TabletState> {
        unimplemented!()
    }
}

impl OsWindow {
    pub fn tablet_state(&self, tablet: DeviceID) -> device::Result<WindowTabletState> {
        unimplemented!()
    }
}
