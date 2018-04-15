extern crate libevdev_sys;
extern crate libudev_sys;

use std::ffi::CStr;
use event::Event;
use hid::{self, ControllerId, ControllerInfo, ControllerAxis, ControllerState, ControllerButton, ButtonState};

pub type UdevDeviceId = i32;

#[derive(Debug)]
pub struct UdevContext {
    pub udev: *mut libudev_sys::udev,
    pub monitor: *mut libudev_sys::udev_monitor,
}

impl Drop for UdevContext {
    fn drop(&mut self) {
        let &mut Self {
            udev, monitor,
        } = self;
        unsafe {
            libudev_sys::udev_monitor_unref(monitor);
            libudev_sys::udev_unref(udev);
        }
    }
}

impl Default for UdevContext {
    fn default() -> Self {
        unsafe {
            let udev = libudev_sys::udev_new();
            let monitor = libudev_sys::udev_monitor_new_from_netlink(udev, b"udev\0".as_ptr() as _);
            let status = libudev_sys::udev_monitor_enable_receiving(monitor);
            Self { udev, monitor }
        }
    }
}


impl UdevContext {
    pub fn poll_next_event(&self) -> Option<Event> {
        let dev = unsafe {
            libudev_sys::udev_monitor_receive_device(self.monitor)
        };
        if dev.is_null() {
            return None;
        }
        let action = unsafe {
            match CStr::from_ptr(libudev_sys::udev_device_get_action(dev)).to_bytes() {
                b"add" => (),
                b"remove" => (),
                b"change" => (),
                b"online" => (),
                b"offline" => (),
                _ => (),
            }
        };
        unsafe {
            libudev_sys::udev_device_unref(dev);
        }
        unimplemented!{}
    }
    pub fn controllers(&self) -> hid::Result<Vec<ControllerId>> {
        unimplemented!{}
    }
    pub fn controller_info(&self, controller: ControllerId) -> hid::Result<ControllerInfo> {
        unimplemented!{}
    }
    pub fn controller_state(&self, controller: ControllerId) -> hid::Result<ControllerState> {
        unimplemented!{}
    }
    pub fn controller_button_state(&self, controller: ControllerId, button: ControllerButton) -> hid::Result<ButtonState> {
        unimplemented!{}
    }
    pub fn controller_axis_state(&self, controller: ControllerId, axis: ControllerAxis) -> hid::Result<f64> {
        unimplemented!{}
    }
}
