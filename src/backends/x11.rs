use std::rc::Rc;

type XDisplay = ();
type udev_monitor = ();
type XIDevice = ();
type udev_device = ();

#[derive(Debug)]
pub struct BackendContext {
    x_dpy: XDisplay,
    udev_mon: udev_monitor,
}
#[derive(Debug)]
pub struct BackendHid {
    platform_display: Rc<BackendContext>,
    udev_dev: udev_device,
    evdev_fd: i32,
    xi_devices: Vec<XIDevice>,
}

impl BackendHid {
    pub fn is_connected(&self) -> bool {
        unimplemented!{}
    }
}
impl Drop for BackendHid {
    fn drop(&mut self) {
        drop(self.udev_dev);
        drop(self.evdev_fd);
        for dev in &self.xi_devices {
            drop(dev);    
        }
        // Then, platform_display is dropped.
    }
}

