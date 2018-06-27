use std::ops::Range;
use std::path::Path;
use std::collections::HashMap;
use uuid::Uuid as Guid;
use event::EventInstant;
use device::{
    self,
    DeviceID, DeviceInfo, UsbIDs, Bus,
    ControllerInfo, MouseInfo, KeyboardInfo, TouchInfo, TabletInfo,
};
use super::OsContext;

pub mod controller;
pub mod keyboard;
pub mod mouse;
pub mod tablet;

pub mod consts {
    pub const MAX_THUMB_BUTTONS: Option<u32> = None;
    pub const MAX_TOP_BUTTONS: Option<u32> = None;
    pub const MAX_BASE_BUTTONS: Option<u32> = None;
    pub const MAX_NUM_BUTTONS: Option<u32> = None;
    pub const MAX_HAT_AXES: Option<u32> = None;
} 

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum OsDeviceID {
    MainMouse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OsAxisInfo;
#[derive(Debug, Clone, PartialEq)]
pub struct OsDeviceInfo;

impl OsAxisInfo {
    pub fn range(&self) -> Range<f64> { unimplemented!() }
    pub fn driver_dead_zone(&self) -> Option<Range<f64>> { unimplemented!() }
    pub fn advised_dead_zone(&self) -> Option<Range<f64>> { unimplemented!() }
    pub fn resolution_hint(&self) -> Option<f64> { unimplemented!() }
    pub fn driver_noise_filter(&self) -> Option<f64> { unimplemented!() }
}

impl OsDeviceInfo {
    pub fn master(&self) -> Option<DeviceID> { unimplemented!() }
    pub fn parent(&self) -> Option<DeviceID> { unimplemented!() }
    pub fn device_node(&self) -> Option<&Path> { unimplemented!() }
    pub fn name(&self) -> Option<&str> { unimplemented!() }
    pub fn serial(&self) -> Option<&str> { unimplemented!() }
    pub fn usb_ids(&self) -> Option<UsbIDs> { unimplemented!() }
    pub fn vendor_name(&self) -> Option<&str> { unimplemented!() }
    pub fn guid(&self) -> Option<Guid> { unimplemented!() }
    pub fn plug_instant(&self) -> Option<EventInstant> { unimplemented!() }
    pub fn bus(&self) -> Option<Bus> { unimplemented!() }
    pub fn driver_name(&self) -> Option<&str> { unimplemented!() }
    pub fn driver_version(&self) -> Option<&str> { unimplemented!() }
    pub fn is_physical(&self) -> Option<bool> { unimplemented!() }
    pub fn controller(&self) -> Option<&ControllerInfo> { unimplemented!() }
    pub fn mouse(&self) -> Option<&MouseInfo> { unimplemented!() }
    pub fn keyboard(&self) -> Option<&KeyboardInfo> { unimplemented!() }
    pub fn touch(&self) -> Option<&TouchInfo> { unimplemented!() }
    pub fn tablet(&self) -> Option<&TabletInfo> { unimplemented!() }
}

impl OsContext {
    pub fn devices(&self) -> device::Result<HashMap<DeviceID, DeviceInfo>> {
        unimplemented!()
    }
    pub fn ping_device(&self, id: DeviceID) -> device::Result<()> {
        unimplemented!()
    }
}
