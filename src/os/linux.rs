use std::rc::Rc;

type XDisplay = ();
type udev_monitor = ();
type XIDevice = ();
type udev_device = ();

#[derive(Debug)]
pub struct OsContext {
    x_dpy: XDisplay,
    udev_mon: udev_monitor,
}
#[derive(Debug)]
pub struct OsHid {
    platform_display: Rc<OsContext>,
    udev_dev: udev_device,
    evdev_fd: i32,
    xi_devices: Vec<XIDevice>,
}

impl OsHid {
    pub fn is_connected(&self) -> bool {
        unimplemented!{}
    }
}
impl Drop for OsHid {
    fn drop(&mut self) {
        drop(self.udev_dev);
        drop(self.evdev_fd);
        for dev in &self.xi_devices {
            drop(dev);    
        }
        // Then, platform_display is dropped.
    }
}

