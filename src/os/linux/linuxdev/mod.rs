// Very interesting doc:
// https://www.kernel.org/doc/html/v4.12/input/gamepad.html

extern crate libevdev_sys;
extern crate libudev_sys;
extern crate libc as c;

use std::fmt::{self, Display, Formatter};
use std::collections::{HashMap, HashSet, VecDeque};
use std::os::unix::ffi::OsStrExt;
use std::ffi::{CStr, OsStr};
use std::path::PathBuf;
use std::ptr;
use std::mem;
use std::cell::{Cell, RefCell};
use event::{Event, EventInstant};
use os::{OsEventInstant, OsDeviceID};
use device::{self, DeviceID, DeviceInfo, ControllerInfo, ControllerAxis, ControllerState, ControllerButton, ButtonState, Bus, VibrationState, AxisInfo};

use self::c::{c_int, c_uint, c_char};

use nix::{self, errno::{self, Errno}};

use self::libevdev_sys::evdev;
use self::libevdev_sys::evdev::libevdev_read_flag;
use self::libevdev_sys::evdev::libevdev_read_status;
use self::libevdev_sys::linux_input;
use self::libevdev_sys::input_event_codes;


unsafe fn cstr_or_none<'a>(ptr: *const c_char) -> Option<&'a CStr> {
    if ptr.is_null() {
        return None;
    }
    Some(&CStr::from_ptr(ptr))
}

fn remove_quotes_if_any(mut s: String) -> String {
    if s.starts_with("\"") && s.ends_with("\"") {
        s.remove(0);
        s.pop();
    }
    s
}


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct LinuxdevToken(u32);

#[derive(Debug, Hash, PartialEq, Eq)]
struct LinuxdevTokenGenerator(LinuxdevToken);

impl Default for LinuxdevTokenGenerator {
    fn default() -> Self {
         LinuxdevTokenGenerator(LinuxdevToken(0))
    }
}

impl LinuxdevTokenGenerator {
    fn next_token(&mut self) -> LinuxdevToken {
        let next = LinuxdevToken((self.0).0.wrapping_add(1));
        mem::replace(&mut self.0, next)
    }
}

impl Display for LinuxdevToken {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum UdevDeviceAction {
    Add,
    Remove,
    Change,
    Move,
    Online,
    Offline,
    Other(String),
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum LinuxEventAPI {
    Evdev,
    Joydev,
}

#[derive(Debug)]
pub struct LinuxdevContext {
    udev: *mut libudev_sys::udev,
    udev_monitor: *mut libudev_sys::udev_monitor,
    udev_enumerate: *mut libudev_sys::udev_enumerate,
    evdev_controllers: RefCell<HashMap<LinuxdevToken, Linuxdev>>,
    token_generator: RefCell<LinuxdevTokenGenerator>,
    pending_translated_events: RefCell<VecDeque<Event>>,
}

#[derive(Debug, PartialEq)]
struct Linuxdev {
    /// The udev_device handle, which is always valid.
    udev_device: *mut libudev_sys::udev_device,
    /// Should this object drop the udev_device ?
    owns_udev_device: bool,
    udev_props: UdevProps,
    /// Opening the device as a file may fail because of ownership issues.
    /// For instance, opening joysticks should succeed, but opening mice should fail because they
    /// are owned by the X server.
    fd: Option<c_int>,
    fd_has_write_access: bool,
    event_api: Option<LinuxEventAPI>,
    /// A libevdev handle is obtained from an open file descriptor, but this may fail for some
    /// reason.
    evdev: Option<LinuxdevEvdev>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct UdevProps {
    usec_initialized: Option<u64>,
    name: Option<String>,
    parent_name: Option<String>,
    id_usb_driver: Option<String>,
    id_bus: Option<String>,
    id_serial: Option<String>,
    id_model: Option<String>,
    id_vendor: Option<String>,
    id_model_id: Option<u16>,
    id_vendor_id: Option<u16>,
    id_input              : bool,
    id_input_joystick     : bool,
    id_input_accelerometer: bool,
    id_input_key          : bool,
    id_input_keyboard     : bool,
    id_input_mouse        : bool,
    id_input_pointingstick: bool,
    id_input_switch       : bool,
    id_input_tablet       : bool,
    id_input_tablet_pad   : bool,
    id_input_touchpad     : bool,
    id_input_touchscreen  : bool,
    id_input_trackball    : bool,
}

#[derive(Debug, PartialEq)]
struct LinuxdevEvdev {
    libevdev: *mut evdev::libevdev,
    /// The registered Force-Feedback ID for rumble effects, or -1.
    rumble_ff_id: Cell<i16>,
    props: EvdevProps,
    buttons: HashSet<ControllerButton>,
    axes: HashMap<ControllerAxis, AxisInfo>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct EvdevProps {
    name: String,
	driver_version: (u16, u8, u8),
    id_bustype: c_int,
    id_product: u16,
    id_vendor: u16,
    is_a_steering_wheel: bool,
    is_a_gamepad: bool,
    is_a_joystick: bool,
    supports_rumble: bool,
    repeat: Option<EvdevRepeat>,
    max_simultaneous_ff_effects: c_int,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
struct EvdevRepeat {
    delay: c_int,
    period: c_int,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OsControllerInfo {
    is_a_gamepad: bool,
    is_a_joystick: bool,
    is_a_steering_wheel: bool,
    supports_rumble: bool,
    buttons: HashSet<ControllerButton>,
    axes: HashMap<ControllerAxis, AxisInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OsControllerState {
    buttons: HashMap<ControllerButton, ButtonState>,
    axes: HashMap<ControllerAxis, f64>,
}


impl Drop for LinuxdevContext {
    fn drop(&mut self) {
        let &mut Self {
            udev, udev_monitor, udev_enumerate,
            evdev_controllers: _,
            token_generator: _,
            pending_translated_events: _,
        } = self;
        unsafe {
            libudev_sys::udev_enumerate_unref(udev_enumerate);
            libudev_sys::udev_monitor_unref(udev_monitor);
            libudev_sys::udev_unref(udev);
        }
    }
}

impl Drop for Linuxdev {
    fn drop(&mut self) {
        let &mut Self {
            udev_device, owns_udev_device, ref udev_props,
            fd, fd_has_write_access: _, event_api: _,
            ref evdev,
        } = self;
        unsafe {
            if owns_udev_device {
                libudev_sys::udev_device_unref(udev_device);
            }
            if let Some(evdev) = evdev.as_ref() {
                let fd = evdev::libevdev_get_fd(evdev.libevdev);
                if evdev.rumble_ff_id.get() != -1 {
                    let res = ev_ioctl::unregister_ff_effect(fd, evdev.rumble_ff_id.get() as _);
                    match res {
                        Err(nix::Error::Sys(Errno::ENODEV)) | Ok(_) => (),
                        Err(e) => error!("Controller {}: failed to unregister the rumble effect while dropping it! (ioctl() generated {})", udev_props.display(), e),
                    };
                }
                let _ignored_status = evdev::libevdev_free(evdev.libevdev);
            }
            if let Some(fd) = fd {
                c::close(fd);
            }
        }
    }
}


impl Default for LinuxdevContext {
    fn default() -> Self {
        unsafe {
            let udev = libudev_sys::udev_new();
            assert!(!udev.is_null());

            let udev_monitor = libudev_sys::udev_monitor_new_from_netlink(udev, b"udev\0".as_ptr() as _);
            assert!(!udev_monitor.is_null());

            let status = libudev_sys::udev_monitor_enable_receiving(udev_monitor);
            if status < 0 {
                error!("udev_monitor_enable_receiving() returned {}", status);
            }

            let udev_enumerate = libudev_sys::udev_enumerate_new(udev);
            assert!(!udev_enumerate.is_null());

            let status = libudev_sys::udev_enumerate_add_match_subsystem(udev_enumerate, b"input\0".as_ptr() as _);
            if status < 0 {
                error!("udev_enumerate_add_match_subsystem() returned {}", status);
            }

            let mut pending_translated_events = VecDeque::with_capacity(32);
            let mut token_generator = LinuxdevTokenGenerator::default();
            let mut evdev_controllers = HashMap::with_capacity(32);

            for entry in udev_enumerate::scan_devices_iter(udev_enumerate) {
                let _entry_value = libudev_sys::udev_list_entry_get_value(entry);
                let devname = libudev_sys::udev_list_entry_get_name(entry);
                if devname.is_null() {
                    continue; // Should never happen, but better safe than sorry!
                }
                let udev_device = libudev_sys::udev_device_new_from_syspath(udev, devname);
                if udev_device.is_null() {
                    continue; // Same as above
                }
                trace!("Got an udev_list_entry named `{}`", cstr_or_none(devname).unwrap().to_string_lossy());
                let dev = Linuxdev::from_udev_device(FromUdevDevice {
                    udev_device, 
                    owns_udev_device: true,
                    try_open_fd_if_is_a_controller: true,
                });
                trace!("Got device {}", dev.display());
                if dev.is_a_controller_and_evdev_node() {
                    let token = token_generator.next_token();
                    let status = dev.pump_evdev(token, &mut pending_translated_events);
                    let worth_keeping = match status {
                        Err(device::Error::DeviceDisconnected(_)) => false,
                        Err(e) => {
                            warn!("Controller {}: pumping evdev failed: {}", dev.display(), e);
                            true
                        },
                        Ok(()) => true,
                    };
                    if worth_keeping {
                        debug!("Added {} to internal evdev_controllers list (token: {})", dev.display(), token);
                        evdev_controllers.insert(token, dev);
                    }
                }
            }

            Self {
                udev, udev_monitor, udev_enumerate,
                evdev_controllers: RefCell::new(evdev_controllers),
                token_generator: RefCell::new(token_generator),
                pending_translated_events: RefCell::new(pending_translated_events),
            }
        }
    }
}


mod udev_enumerate {
    use super::*;

    #[derive(Debug)]
    pub struct Iter {
        entry: *mut libudev_sys::udev_list_entry,
    }

    impl Iterator for Iter {
        type Item = *mut libudev_sys::udev_list_entry;
        fn next(&mut self) -> Option<Self::Item> {
            if self.entry.is_null() {
                return None;
            }
            let next = unsafe {
                libudev_sys::udev_list_entry_get_next(self.entry)
            };
            Some(mem::replace(&mut self.entry, next))
        }
    }

    pub unsafe fn scan_devices_iter(udev_enumerate: *mut libudev_sys::udev_enumerate) -> Iter {
        assert!(!udev_enumerate.is_null());
        let status = libudev_sys::udev_enumerate_scan_devices(udev_enumerate);
        if status < 0 {
            error!("udev_enumerate_scan_devices() returned {}", status);
        }
        let entry = libudev_sys::udev_enumerate_get_list_entry(udev_enumerate);
        Iter { entry }
    }
}

impl UdevDeviceAction {
    pub fn from_cstr(action: &CStr) -> Self {
        match action.to_bytes_with_nul() {
            b"add\0" => UdevDeviceAction::Add,
            b"remove\0" => UdevDeviceAction::Remove,
            b"move\0" => UdevDeviceAction::Move,
            b"change\0" => UdevDeviceAction::Change,
            b"online\0" => UdevDeviceAction::Online,
            b"offline\0" => UdevDeviceAction::Offline,
            _ => UdevDeviceAction::Other(action.to_string_lossy().into_owned()),
        }
    }
}


impl LinuxdevContext {
    pub fn poll_next_event(&self) -> Option<Event> {
        self.pump_events();

        let ev = self.pending_translated_events.borrow_mut().pop_front();
        if let Some(&Event::DeviceDisconnected { device: DeviceID(OsDeviceID::Linuxdev(token)), .. }) = ev.as_ref() {
            let dev = self.evdev_controllers.borrow_mut().remove(&token).unwrap();
            debug!("Removed disconnected {} from internal evdev_controllers list (token: {})", dev.display(), token);
        }
        ev
    }
    fn pump_events(&self) {
        for (token, dev) in self.evdev_controllers.borrow().iter() {
            let status = dev.pump_evdev(*token, &mut self.pending_translated_events.borrow_mut());
            match status {
                // If disconnected, don't do anything; let pump_udev_monitor() handle stuff.
                // Further actions with the fd will fail and it's OK because everyone handles this
                // in their own way.
                Err(device::Error::DeviceDisconnected(_)) | Ok(()) => (), 
                Err(e) => warn!("Controller {}: pumping evdev failed: {}", dev.display(), e),
            };
        }
        // We want to pump existing devices _before_ learning that they have been
        // disconnected (all events matter), so pump the udev_monitor last. Any newly added device
        // will also be pumped immediately anyway.
        self.pump_udev_monitor();
    }
    fn pump_udev_monitor(&self) {
        loop {
            let udev_device = unsafe {
                libudev_sys::udev_monitor_receive_device(self.udev_monitor)
            };
            if udev_device.is_null() {
                break;
            }
            let action = unsafe {
                cstr_or_none(libudev_sys::udev_device_get_action(udev_device))
            };
            let action = match action {
                None => unsafe {
                    libudev_sys::udev_device_unref(udev_device);
                    continue
                },
                Some(action) => UdevDeviceAction::from_cstr(action),
            };
            match action {
                UdevDeviceAction::Add => self.add_linuxdev(unsafe {
                    // Increment refcount, because always decreased at the end
                    let udev_device = libudev_sys::udev_device_ref(udev_device);
                    Linuxdev::from_udev_device(FromUdevDevice {
                        udev_device, 
                        owns_udev_device: true,
                        try_open_fd_if_is_a_controller: true,
                    })
                }),
                UdevDeviceAction::Remove => self.on_udev_device_removed(udev_device),
                  UdevDeviceAction::Move
                | UdevDeviceAction::Change
                | UdevDeviceAction::Online
                | UdevDeviceAction::Offline
                | UdevDeviceAction::Other(_) => {
                    warn!("Ignoring {:?}", action);
                }
            };
            unsafe {
                libudev_sys::udev_device_unref(udev_device);
            }
        }
    }
    fn add_linuxdev(&self, dev: Linuxdev) {
        if !dev.is_a_controller_and_evdev_node() {
            return;
        }
        let token = self.token_generator.borrow_mut().next_token();
        let device_connected_event = Event::DeviceConnected {
            device: DeviceID(OsDeviceID::Linuxdev(token)),
            instant: dev.plug_instant(),
            info: dev.device_info(),
        };
        let mut queue = self.pending_translated_events.borrow_mut();
        queue.push_back(device_connected_event);
        let status = dev.pump_evdev(token, &mut queue);
        match status {
            Err(device::Error::DeviceDisconnected(_)) | Ok(()) => (), 
            Err(e) => warn!("Controller {}: pumping evdev failed: {}", dev.display(), e),
        };
        // Still add it even though the device could have been disconnected.
        // Wait until we receive the message from udev to do things properly.
        debug!("Added newly connected {} to internal evdev_controllers list (token: {})", dev.display(), token);
        self.evdev_controllers.borrow_mut().insert(token, dev);
    }
    fn on_udev_device_removed(&self, udev_device: *mut libudev_sys::udev_device) {
        // Reverse lookup
        let target_devnode = unsafe {
            Linuxdev::device_node_pathbuf_of_udev_device(udev_device)
        };
        if target_devnode.is_none() {
            return; // Can't do anything about it
        }
        let target_devnode = target_devnode.unwrap();
        let token = self.evdev_controllers.borrow().iter().filter_map(|(token, dev)| {
            let devnode = unsafe {
                Linuxdev::device_node_pathbuf_of_udev_device(dev.udev_device)
            };
            devnode.map(|pathbuf| if pathbuf == target_devnode {
                Some(*token)
            } else {
                None
            }).unwrap_or(None)
        }).next();

        if token.is_none() {
            return; // It's fine; the udev_device is not necessarily a controller!
        }
        // NOTE: Don't remove the device from our list, yet !
        // Wait until the DeviceDisconnected event is reported to the user to do it.
        // See self.poll_next_event()
        let token = token.unwrap();
        let dev = &self.evdev_controllers.borrow()[&token];
        let device_disconnected_event = Event::DeviceDisconnected {
            device: DeviceID(OsDeviceID::Linuxdev(token)),
            instant: dev.instant_now(), // Looks like it's the closest we can get... ._.
        };
        self.pending_translated_events.borrow_mut().push_back(device_disconnected_event);
        debug!("{} disconnected but still kept in internal evdev_controllers list (token: {})", dev.display(), token);
    }
    pub fn controllers(&self) -> device::Result<HashMap<DeviceID, DeviceInfo>> {
		// We are not required to rescan devices via e.g udev_enumerate_scan_devices.
		// We're in sync with the event queue and this is fine.
        Ok(self.evdev_controllers.borrow().iter().map(|(token, dev)| {
            let id = DeviceID(OsDeviceID::Linuxdev(*token));
            let info = dev.device_info();
            (id, info)
        }).collect())
    }
    pub fn ping_controller(&self, token: LinuxdevToken) -> device::Result<()> {
        match self.evdev_controllers.borrow().get(&token) {
            None => device::disconnected(),
            Some(dev) => dev.pump_evdev(token, &mut self.pending_translated_events.borrow_mut()),
        }
    }
    pub fn controller_state(&self, controller: DeviceID) -> device::Result<ControllerState> {
        self.with_controller(controller, |dev| dev.controller_state().map(ControllerState))
    }
    pub fn controller_button_state(&self, controller: DeviceID, button: ControllerButton) -> device::Result<ButtonState> {
        self.with_controller(controller, |dev| dev.controller_button_state(button))
    }
    pub fn controller_axis_state(&self, controller: DeviceID, axis: ControllerAxis) -> device::Result<f64> {
        self.with_controller(controller, |dev| dev.controller_axis_state(axis))
    }
    pub fn controller_set_vibration(&self, controller: DeviceID, vibration: &VibrationState) -> device::Result<()> {
        self.with_controller(controller, |dev| dev.controller_set_vibration(vibration))
    }
    // We take a closure because we can't return a reference to the DeviceID (it outlives the
    // borrow() of self.evdev_controllers).
    fn with_controller<T, F: FnMut(&Linuxdev) -> device::Result<T>>(&self, controller: DeviceID, mut f: F) -> device::Result<T> {
        if let OsDeviceID::Linuxdev(token) = controller.0 {
            if let Some(dev) = self.evdev_controllers.borrow().get(&token) {
                debug_assert!(dev.is_a_controller_and_evdev_node());
                f(dev)
            } else {
                device::disconnected()    
            }
        } else {
            // Don't panic here. Sometimes the higher layer just passes the DeviceID as-is
            // to use without checking if it is adressed to us in the first place.
            // E.g user calls `controller_axis_state(some_device_that_is_not_a_controller)`.
            device::not_supported_by_device("This device does not refer to a controller")
        }
    }
}


impl OsControllerInfo {
    pub fn is_a_gamepad(&self) -> bool {
        self.is_a_gamepad
    }
    pub fn is_a_joystick(&self) -> bool {
        self.is_a_joystick
    }
    pub fn is_a_steering_wheel(&self) -> bool {
        self.is_a_steering_wheel
    }
    pub fn supports_rumble(&self) -> bool {
        self.supports_rumble
    }
    pub fn has_button(&self, button: ControllerButton) -> bool {
        self.buttons.contains(&button)
    }
    pub fn has_axis(&self, axis: ControllerAxis) -> bool {
        self.axes.contains_key(&axis)
    }
    pub fn axis(&self, axis: ControllerAxis) -> Option<AxisInfo> {
        self.axes.get(&axis).map(Clone::clone)
    }
}
impl OsControllerState {
    pub fn button(&self, button: ControllerButton) -> Option<ButtonState> {
        self.buttons.get(&button).map(Clone::clone)
    }
    pub fn axis(&self, axis: ControllerAxis) -> Option<f64> {
        self.axes.get(&axis).map(Clone::clone)
    }
}


// For logging. This is very simple right now.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LinuxdevDisplay<'a> {
    name: &'a str,
}

impl<'a> Display for LinuxdevDisplay<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let &Self { ref name } = self;
        write!(f, "`{}`", name)
    }
}

impl EvdevProps {
    pub fn display<'a>(&'a self) -> LinuxdevDisplay<'a> {
        LinuxdevDisplay { name: self.name.as_str() }
    }
}

impl UdevProps {
    pub fn display<'a>(&'a self) -> LinuxdevDisplay<'a> {
        LinuxdevDisplay { name: self.name().unwrap_or("???") }
    }
}

impl Linuxdev {
    pub fn display<'a>(&'a self) -> LinuxdevDisplay<'a> {
        self.evdev.as_ref().map(|e| e.props.display()).unwrap_or(self.udev_props.display())
    }
}




impl UdevProps {
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(String::as_str).or(self.parent_name.as_ref().map(String::as_str))
    }
    pub fn is_a_controller(&self) -> bool {
        self.id_input_joystick
    }
}
impl Linuxdev {
    pub fn name(&self) -> Option<&str> {
        self.evdev.as_ref().map(|e| e.props.name.as_str()).or(self.udev_props.name())
    }
    pub fn product_id(&self) -> Option<u16> {
        self.evdev.as_ref().map(|e| e.props.id_product).or(self.udev_props.id_model_id)
    }
    pub fn vendor_id(&self) -> Option<u16> {
        self.evdev.as_ref().map(|e| e.props.id_vendor).or(self.udev_props.id_vendor_id)
    }
    pub fn driver_name(&self) -> Option<String> {
        self.udev_props.id_usb_driver.clone()
    }
    pub fn driver_version(&self) -> Option<String> {
		self.evdev.as_ref().map(|e| {
			let (major, minor, patch) = e.props.driver_version;
			format!("{}.{}.{}", major, minor, patch)
		})
    }
    pub fn bus(&self) -> Option<Bus> {
        let bus_via_evdev = self.evdev.as_ref().map(|e| match e.props.id_bustype {
            linux_bus_id::BUS_PCI         => Some(Bus::Pci),
            linux_bus_id::BUS_ISAPNP      => None,
            linux_bus_id::BUS_USB         => Some(Bus::Usb),
            linux_bus_id::BUS_HIL         => None,
            linux_bus_id::BUS_BLUETOOTH   => Some(Bus::Bluetooth),
            linux_bus_id::BUS_VIRTUAL     => Some(Bus::Virtual),
            linux_bus_id::BUS_ISA         => None,
            linux_bus_id::BUS_I8042       => None,
            linux_bus_id::BUS_XTKBD       => None,
            linux_bus_id::BUS_RS232       => None,
            linux_bus_id::BUS_GAMEPORT    => None,
            linux_bus_id::BUS_PARPORT     => None,
            linux_bus_id::BUS_AMIGA       => None,
            linux_bus_id::BUS_ADB         => None,
            linux_bus_id::BUS_I2C         => None,
            linux_bus_id::BUS_HOST        => None,
            linux_bus_id::BUS_GSC         => None,
            linux_bus_id::BUS_ATARI       => None,
            linux_bus_id::BUS_SPI         => None,
            linux_bus_id::BUS_RMI         => None,
            linux_bus_id::BUS_CEC         => None,
            linux_bus_id::BUS_INTEL_ISHTP => None,
            _ => None,
        }).unwrap_or(None);

        let bus_via_udev = self.udev_props.id_bus.as_ref().map(|s| match s.as_str() {
            "usb" => Some(Bus::Usb),
            "pci" => Some(Bus::Pci),
            "bluetooth" => Some(Bus::Bluetooth),
            "virtual" => Some(Bus::Virtual),
            _ => None,
        }).unwrap_or(None);

        bus_via_evdev.or(bus_via_udev)
    }
    pub fn is_a_controller_and_evdev_node(&self) -> bool {
        self.is_a_controller() && self.event_api == Some(LinuxEventAPI::Evdev)
    }
    pub fn is_a_controller(&self) -> bool {
        self.udev_props.is_a_controller()
    }
    pub fn is_a_gamepad(&self) -> bool {
        self.is_a_controller() && self.evdev.as_ref().map(|e| e.props.is_a_gamepad).unwrap_or(false)
    }
    pub fn is_a_steering_wheel(&self) -> bool {
        self.is_a_controller() && self.evdev.as_ref().map(|e| e.props.is_a_steering_wheel).unwrap_or(false)
    }
    pub fn is_a_joystick(&self) -> bool {
        self.is_a_controller() && self.evdev.as_ref().map(|e| e.props.is_a_joystick).unwrap_or(false)
    }
    pub fn supports_rumble(&self) -> bool {
        // self.evdev.is_some() implies self.fd.is_some()
        self.fd_has_write_access && self.evdev.as_ref().map(|e| e.props.supports_rumble).unwrap_or(false)
    }
    pub unsafe fn device_node_pathbuf_of_udev_device(udev_device: *mut libudev_sys::udev_device) -> Option<PathBuf> {
		cstr_or_none(libudev_sys::udev_device_get_devnode(udev_device)).map(|cstr| OsStr::from_bytes(cstr.to_bytes()).into())
    }
    pub fn device_info(&self) -> DeviceInfo {
        let evdev = self.evdev.as_ref().unwrap();
        DeviceInfo {
            master: None,
            parent: None, // This is more complicated than it looks and I doubt people do care.
            device_node: unsafe {
                Self::device_node_pathbuf_of_udev_device(self.udev_device)
			},
            name: self.name().map(|s| s.to_owned()),
            serial: self.udev_props.id_serial.clone(),
            usb_product_info: match (self.vendor_id(), self.product_id()) {
                (Some(vendor_id), Some(product_id)) => Some(device::UsbProductInfo {
                    vendor_id,
                    product_id,
                    vendor_name: self.udev_props.id_vendor.clone(),
                    product_name: None,
                }),
                _ => None,
            },
            guid: None,
            plug_instant: Some(self.plug_instant()),
            bus: self.bus(),
            driver_name: self.driver_name(),
            driver_version: self.driver_version(),
            is_physical: None, // WISH: I could actually investigate this
            controller: Some(ControllerInfo(OsControllerInfo {
                is_a_gamepad: self.is_a_gamepad(),
                is_a_joystick: self.is_a_joystick(),
                is_a_steering_wheel: self.is_a_steering_wheel(),
                supports_rumble: self.supports_rumble(),
                buttons: evdev.buttons.clone(),
                axes: evdev.axes.clone(),
            })),
            mouse: None,
            keyboard: None,
            touch: None,
            tablet: None,
        }
    }
}

struct FromUdevDevice {
    udev_device: *mut libudev_sys::udev_device,
    owns_udev_device: bool,
    try_open_fd_if_is_a_controller: bool,
}

impl Linuxdev {
    unsafe fn from_udev_device(params: FromUdevDevice) -> Self {
        let FromUdevDevice {
            udev_device, owns_udev_device, try_open_fd_if_is_a_controller,
        } = params;

        assert!(!udev_device.is_null());

        // --- Getting as much info as we can
        //
        // Rationale :
        // - It's convenient to do everything in one place;
        // - Getting infos is a somewhat annoying process;
        //   We ease our lives by doing it once and storing what we care about in
        //   a representation that is convenient for us.
        //
        // The drawback is that we're eagerly reserving some memory for stuff the caller
        // might never actually care about. But hey, system event queues already eat up a bunch of memory in any case.

        let udev_prop_of = |udev_device, name: &[u8]| -> Option<&CStr> {
            assert_eq!(b'\0', *name.last().unwrap());
            cstr_or_none(libudev_sys::udev_device_get_property_value(udev_device, name.as_ptr() as _))
        };
        let udev_prop = |name: &[u8]| udev_prop_of(udev_device, name);
        let udev_parent_prop = |name: &[u8]| -> Option<&CStr> {
            // NOTE: Linked to child device, no need to free it.
            let parent = libudev_sys::udev_device_get_parent(udev_device);
            if parent.is_null() {
                None
            } else {
                udev_prop_of(parent, name)
            }
        };
        let udev_prop_bool = |name: &[u8]| udev_prop(name).map(|s| s.to_bytes()[0] == b'1').unwrap_or(false);
        let udev_prop_string = |name: &[u8]| udev_prop(name).map(|s| s.to_string_lossy().into_owned());
        let udev_parent_prop_string = |name: &[u8]| udev_parent_prop(name).map(|s| s.to_string_lossy().into_owned());

        let udev_props = UdevProps {
            usec_initialized: udev_prop_string(b"USEC_INITIALIZED\0").map(|s| s.parse().unwrap()),
            id_usb_driver: udev_prop_string(b"ID_USB_DRIVER\0"),
            id_bus: udev_prop_string(b"ID_BUS\0"),
            id_serial: udev_prop_string(b"ID_SERIAL\0"),
            id_model: udev_prop_string(b"ID_MODEL\0"), // "Controller" ??
            id_vendor: udev_prop_string(b"ID_VENDOR\0"),
            id_model_id : udev_prop_string(b"ID_MODEL_ID\0") .map(|s| u16::from_str_radix(&s, 16).unwrap()),
            id_vendor_id: udev_prop_string(b"ID_VENDOR_ID\0").map(|s| u16::from_str_radix(&s, 16).unwrap()),
            name: udev_prop_string(b"NAME\0").map(remove_quotes_if_any),
            parent_name: udev_parent_prop_string(b"NAME\0").map(remove_quotes_if_any),
            // NOTE: from udev source (https://github.com/systemd/systemd).
            id_input              : udev_prop_bool(b"ID_INPUT\0"),
            id_input_joystick     : udev_prop_bool(b"ID_INPUT_JOYSTICK\0"),
            id_input_accelerometer: udev_prop_bool(b"ID_INPUT_ACCELEROMETER\0"),
            id_input_key          : udev_prop_bool(b"ID_INPUT_KEY\0"),
            id_input_keyboard     : udev_prop_bool(b"ID_INPUT_KEYBOARD\0"),
            id_input_mouse        : udev_prop_bool(b"ID_INPUT_MOUSE\0"),
            id_input_pointingstick: udev_prop_bool(b"ID_INPUT_POINTINGSTICK\0"),
            id_input_switch       : udev_prop_bool(b"ID_INPUT_SWITCH\0"),
            id_input_tablet       : udev_prop_bool(b"ID_INPUT_TABLET\0"),
            id_input_tablet_pad   : udev_prop_bool(b"ID_INPUT_TABLET_PAD\0"),
            id_input_touchpad     : udev_prop_bool(b"ID_INPUT_TOUCHPAD\0"),
            id_input_touchscreen  : udev_prop_bool(b"ID_INPUT_TOUCHSCREEN\0"),
            id_input_trackball    : udev_prop_bool(b"ID_INPUT_TRACKBALL\0"),
        };

        let devnode = cstr_or_none(libudev_sys::udev_device_get_devnode(udev_device));

        let event_api = devnode.map(|devnode| {
            let devnode = devnode.to_str().unwrap();
            let last_slash = devnode.rfind('/').unwrap();
            if devnode[last_slash..].starts_with("/event") {
                Some(LinuxEventAPI::Evdev)
            } else if devnode[last_slash..].starts_with("/js") {
                Some(LinuxEventAPI::Joydev)
            } else {
                None
            }
        }).unwrap_or(None);

        let (fd, fd_has_write_access) = {
            // Don't even try if it's not a controller. All other device kinds are "owned" by the X
            // server so we're normally not allowed to open them.
            if devnode.is_none() || !try_open_fd_if_is_a_controller || !udev_props.is_a_controller() {
                (None, false)
            } else {
                let devnode = devnode.unwrap();
                // O_RDWR for the ability to write force feedback events;
                // O_NONBLOCK so reading events doesn't block (libevdev assumes this by default.
                //    Changing is possible, but requires calling some APIs).
                let fd = c::open(devnode.as_ptr(), c::O_RDWR | c::O_NONBLOCK, 0666);
                if fd != -1 {
                    (Some(fd), true)
                } else {
                    let devnode_str = devnode.to_string_lossy();
                    let controller_name = udev_props.display();
                    warn!("Could not open {} (controller {}) for reading and writing! (errno: {})", devnode_str, controller_name, Errno::last());

                    // But not all hope is lost. Read-only, maybe?
                    let fd = c::open(devnode.as_ptr(), c::O_RDONLY | c::O_NONBLOCK, 0666);
                    if fd != -1 {
                        (Some(fd), false)
                    } else {
                        error!("Could not open {} (controller {}) for reading! (errno: {})", devnode_str, controller_name, Errno::last());
                        (None, false)
                    }
                }
            }
        };

        let evdev = event_api.map(|event_api| match event_api {
            LinuxEventAPI::Joydev => None,
            LinuxEventAPI::Evdev => fd.map(|fd| {
                let mut libevdev = ptr::null_mut();
                let status = evdev::libevdev_new_from_fd(fd, &mut libevdev);
                if status < 0 {
                    warn!("Controller {}: libevdev_new_from_fd() returned {}", udev_props.display(), status);
                    None
                } else {
                    Some(LinuxdevEvdev::from_libevdev(libevdev))
                }
            }).unwrap_or(None),
        }).unwrap_or(None);

        let mut dev = Self {
            udev_device, owns_udev_device, udev_props,
            fd, fd_has_write_access, event_api,
            evdev,
        };
        if dev.evdev.is_some() {
            dev.evdev_refresh_all_controller_axes_support();
            dev.evdev_refresh_all_controller_buttons_support();
        }
        dev
    }

    fn parent(&self, try_open_fd_if_is_a_controller: bool) -> Option<Linuxdev> {
        let parent = unsafe {
            libudev_sys::udev_device_get_parent(self.udev_device)
        };
        if parent.is_null() {
            None
        } else {
            Some(unsafe { Self::from_udev_device(FromUdevDevice {
                udev_device: parent,
                owns_udev_device: false,
                try_open_fd_if_is_a_controller,
            })})
        }
    }

    pub fn plug_usecs(&self) -> u64 {
        match self.udev_props.usec_initialized {
            Some(usecs) => usecs,
            _ => {
                error!("Controller {}: USEC_INITIALIZED property wasn't set by udev; this should never happen", self.display());
                0
            },
        }
    }
    pub fn current_usecs_since_initialized(&self) -> u64 {
        unsafe {
            libudev_sys::udev_device_get_usec_since_initialized(self.udev_device)
        }
    }
    pub fn usecs_now(&self) -> u64 {
        self.plug_usecs().saturating_add(self.current_usecs_since_initialized())
    }
    pub fn plug_instant(&self) -> EventInstant {
        EventInstant(OsEventInstant::UdevUsecs(self.plug_usecs()))
    }
    pub fn instant_now(&self) -> EventInstant {
        EventInstant(OsEventInstant::UdevUsecs(self.usecs_now()))
    }
}

impl LinuxdevEvdev {
    unsafe fn from_libevdev(libevdev: *mut evdev::libevdev) -> Self {
        assert!(!libevdev.is_null());
        let props = EvdevProps {
            name      : {
                let cstr = cstr_or_none(evdev::libevdev_get_name(libevdev));
                let name = cstr.map(|cstr| remove_quotes_if_any(cstr.to_string_lossy().into_owned()));
                name.unwrap_or_else(|| "???".to_owned())
            },
			driver_version: {
				let v = evdev::libevdev_get_driver_version(libevdev);
				if v == -1 {
					(0, 0, 0) // Device disconnected???
				} else {
					let major = v >> 16;
					let minor = (v >> 8) & 0xff;
					let patch = v & 0xff;
					(major as _, minor as _, patch as _)
				}
			},
            id_bustype: evdev::libevdev_get_id_bustype(libevdev),
            id_product: evdev::libevdev_get_id_product(libevdev) as _,
            id_vendor : evdev::libevdev_get_id_vendor(libevdev) as _,
            is_a_steering_wheel: {
                let has_abs_wheel = 0 != evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_ABS as _, input_event_codes::ABS_WHEEL as _);
                let has_btn_wheel = 0 != evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_WHEEL as _);
                has_abs_wheel || has_btn_wheel
            },
            is_a_gamepad : 0 != evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_GAMEPAD as _),
            is_a_joystick: 0 != evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_JOYSTICK as _),
            supports_rumble: 0 != evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_FF as _, ff::FF_RUMBLE as _),
            // There's also BTN_MOUSE, BTN_DIGI, etc... but the X server has ownership, not us.
            repeat: {
                let (mut delay, mut period) = (0, 0);
                let status = evdev::libevdev_get_repeat(libevdev, &mut delay, &mut period);
                if status < 0 {
                    None
                } else {
                    Some(EvdevRepeat { delay, period })
                }
            },
            max_simultaneous_ff_effects: {
                let fd = evdev::libevdev_get_fd(libevdev);
                assert_ne!(fd, -1);
                let mut max_simultaneous_ff_effects = 0;
                let status = ev_ioctl::max_simultaneous_ff_effects(fd, &mut max_simultaneous_ff_effects);
                match status {
                    Ok(_) => max_simultaneous_ff_effects,
                    Err(e) => {
                        warn!("EVIOCGEFFECTS ioctl() returned {}", e);
                        0
                    },
                }
            },
        };

        Self {
            libevdev, props,
            rumble_ff_id: Cell::new(-1),
            // Filled later by the device. The reason is, the mapping depends
            // on the controller kind, which we aren't sure of until the Linuxdev
            // object is wholly created.
            buttons: Default::default(),
            axes: Default::default(),
        }
    }
}

fn axis_info_from_linux_absinfo(absinfo: &linux_input::input_absinfo) -> AxisInfo {
    let &linux_input::input_absinfo {
        value: _, minimum, maximum, fuzz, resolution, flat,
    } = absinfo;

    AxisInfo {
        range: minimum as f64 .. maximum as f64,
        dead_zone: {
            let max = flat.abs() as f64;
            Some(-max .. max)
        },
        fuzz: fuzz as f64,
        resolution: resolution as f64,
    }
}


impl Linuxdev {
    pub fn translate_ev_key(&self, code: u16) -> ControllerButton {
        controller_button_from_ev_key_code(code)
    }
    pub fn untranslate_ev_key(&self, button: ControllerButton) -> Option<u16> {
        controller_button_to_ev_key_code(button)
    }
    pub fn translate_ev_abs(&self, code: u16) -> ControllerAxis {
        if self.is_a_joystick() {
            controller_axis_from_ev_abs_code_for_joysticks(code)
        } else {
            controller_axis_from_ev_abs_code_for_gamepads_or_steering_wheels(code)
        }
    }
    pub fn untranslate_ev_abs(&self, axis: ControllerAxis) -> Option<u16> {
        if self.is_a_joystick() {
            controller_axis_to_ev_abs_code_for_joysticks(axis)
        } else {
            controller_axis_to_ev_abs_code_for_gamepads_or_steering_wheels(axis)
        }
    }

    pub fn evdev_refresh_all_controller_buttons_support(&mut self) {
        let buttons = self.evdev_all_controller_buttons_support();
        self.evdev.as_mut().unwrap().buttons = buttons;
    }
    pub fn evdev_refresh_all_controller_axes_support(&mut self) {
        let axes = self.evdev_all_controller_axes_support();
        self.evdev.as_mut().unwrap().axes = axes;
    }


    pub fn evdev_all_controller_buttons_support(&self) -> HashSet<ControllerButton> {
        let libevdev = self.evdev.as_ref().unwrap().libevdev;
        let mut all_buttons = HashSet::with_capacity(ALL_BTN_CODES.len());
        for code in ALL_BTN_CODES {
            let has_it = 0 != unsafe {
                evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, *code as _)
            };
            if has_it {
                all_buttons.insert(self.translate_ev_key(*code));
            }
        }
        all_buttons
    }
    pub fn evdev_all_controller_buttons_state(&self) -> HashMap<ControllerButton, ButtonState> {
        let mut all_buttons = HashMap::with_capacity(ALL_BTN_CODES.len());
        for code in ALL_BTN_CODES {
            let button = self.translate_ev_key(*code);
            if let Some(state) = self.evdev_controller_button_state(button) {
                all_buttons.insert(button, state);
            }
        }
        all_buttons
    }
    pub fn evdev_controller_button_state(&self, button: ControllerButton) -> Option<ButtonState> {
        let mut value = 0;
        let status = unsafe {
            let libevdev = self.evdev.as_ref().unwrap().libevdev;
            let type_ = input_event_codes::EV_KEY;
            let code = self.untranslate_ev_key(button)?;
            evdev::libevdev_fetch_event_value(libevdev, type_ as _, code as _, &mut value)
        };
        match status {
            0 => None,
            _ => Some(match value {
                0 => ButtonState::Up,
                _ => ButtonState::Down,
            }),
        }
    }

    pub fn evdev_all_controller_axes_support(&self) -> HashMap<ControllerAxis, AxisInfo> {
        let libevdev = self.evdev.as_ref().unwrap().libevdev;
        let mut all_axes = HashMap::with_capacity(ALL_ABS_CODES.len());
        for code in ALL_ABS_CODES {
            let has_it = 0 != unsafe {
                evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_ABS as _, *code as _)
            };
            if has_it {
                let absinfo = unsafe {
                    evdev::libevdev_get_abs_info(libevdev, *code as _)
                };
                if !absinfo.is_null() {
                    all_axes.insert(self.translate_ev_abs(*code), axis_info_from_linux_absinfo(unsafe { &*absinfo }));
                }
            }
        }
        all_axes
    }
    pub fn evdev_all_controller_axes_state(&self) -> HashMap<ControllerAxis, f64> {
        let mut all_axes = HashMap::with_capacity(ALL_ABS_CODES.len());
        for code in ALL_ABS_CODES {
            let axis = self.translate_ev_abs(*code);
            if let Some(state) = self.evdev_controller_axis_state(axis) {
                all_axes.insert(axis, state);
            }
        }
        all_axes
    }
    pub fn evdev_controller_axis_state(&self, axis: ControllerAxis) -> Option<f64> {
        let libevdev = self.evdev.as_ref().unwrap().libevdev;
        let mut value = 0;
        let status = unsafe {
            let type_ = input_event_codes::EV_ABS;
            let code = self.untranslate_ev_abs(axis)?;
            evdev::libevdev_fetch_event_value(libevdev, type_ as _, code as _, &mut value)
        };
        match status {
            0 => None,
            _ => Some(value as f64),
        }
    }
}

static ALL_BTN_CODES: &[u16] = &[
    input_event_codes::BTN_GEAR_DOWN ,
    input_event_codes::BTN_GEAR_UP   ,
    input_event_codes::BTN_TRIGGER   ,
    input_event_codes::BTN_THUMB     ,
    input_event_codes::BTN_THUMB2    ,
    input_event_codes::BTN_TOP       ,
    input_event_codes::BTN_TOP2      ,
    input_event_codes::BTN_PINKIE    ,
    input_event_codes::BTN_DEAD      ,
    input_event_codes::BTN_BASE      ,
    input_event_codes::BTN_BASE2     ,
    input_event_codes::BTN_BASE3     ,
    input_event_codes::BTN_BASE4     ,
    input_event_codes::BTN_BASE5     ,
    input_event_codes::BTN_BASE6     ,
    input_event_codes::BTN_A         ,
    input_event_codes::BTN_B         ,
    input_event_codes::BTN_C         ,
    input_event_codes::BTN_X         ,
    input_event_codes::BTN_Y         ,
    input_event_codes::BTN_Z         ,
    input_event_codes::BTN_TL        ,
    input_event_codes::BTN_TR        ,
    input_event_codes::BTN_TL2       ,
    input_event_codes::BTN_TR2       ,
    input_event_codes::BTN_SELECT    ,
    input_event_codes::BTN_START     ,
    input_event_codes::BTN_MODE      ,
    input_event_codes::BTN_THUMBL    ,
    input_event_codes::BTN_THUMBR    ,
    input_event_codes::BTN_DPAD_UP   ,
    input_event_codes::BTN_DPAD_DOWN ,
    input_event_codes::BTN_DPAD_LEFT ,
    input_event_codes::BTN_DPAD_RIGHT,
    input_event_codes::BTN_0         ,
    input_event_codes::BTN_1         ,
    input_event_codes::BTN_2         ,
    input_event_codes::BTN_3         ,
    input_event_codes::BTN_4         ,
    input_event_codes::BTN_5         ,
    input_event_codes::BTN_6         ,
    input_event_codes::BTN_7         ,
    input_event_codes::BTN_8         ,
    input_event_codes::BTN_9         ,
];

static ALL_ABS_CODES: &[u16] = &[
    input_event_codes::ABS_THROTTLE,
    input_event_codes::ABS_RUDDER  ,
    input_event_codes::ABS_WHEEL   ,
    input_event_codes::ABS_GAS     ,
    input_event_codes::ABS_BRAKE   ,
    input_event_codes::ABS_HAT0X   ,
    input_event_codes::ABS_HAT0Y   ,
    input_event_codes::ABS_HAT1X   ,
    input_event_codes::ABS_HAT1Y   ,
    input_event_codes::ABS_HAT2X   ,
    input_event_codes::ABS_HAT2Y   ,
    input_event_codes::ABS_HAT3X   ,
    input_event_codes::ABS_HAT3Y   ,
    input_event_codes::ABS_X       ,
    input_event_codes::ABS_Y       ,
    input_event_codes::ABS_Z       ,
    input_event_codes::ABS_RX      ,
    input_event_codes::ABS_RY      ,
    input_event_codes::ABS_RZ      ,
];

fn controller_button_to_ev_key_code(button: ControllerButton) -> Option<u16> {
    match button {
        ControllerButton::GearDown    => Some(input_event_codes::BTN_GEAR_DOWN ),
        ControllerButton::GearUp      => Some(input_event_codes::BTN_GEAR_UP   ),
        ControllerButton::Trigger     => Some(input_event_codes::BTN_TRIGGER   ),
        ControllerButton::Thumb(0)    => Some(input_event_codes::BTN_THUMB     ),
        ControllerButton::Thumb(1)    => Some(input_event_codes::BTN_THUMB2    ),
        ControllerButton::Thumb(_)    => None,
        ControllerButton::Top(0)      => Some(input_event_codes::BTN_TOP       ),
        ControllerButton::Top(1)      => Some(input_event_codes::BTN_TOP2      ),
        ControllerButton::Top(_)      => None,
        ControllerButton::Pinkie      => Some(input_event_codes::BTN_PINKIE    ),
        ControllerButton::Dead        => Some(input_event_codes::BTN_DEAD      ),
        ControllerButton::Base(0)     => Some(input_event_codes::BTN_BASE      ),
        ControllerButton::Base(1)     => Some(input_event_codes::BTN_BASE2     ),
        ControllerButton::Base(2)     => Some(input_event_codes::BTN_BASE3     ),
        ControllerButton::Base(3)     => Some(input_event_codes::BTN_BASE4     ),
        ControllerButton::Base(4)     => Some(input_event_codes::BTN_BASE5     ),
        ControllerButton::Base(5)     => Some(input_event_codes::BTN_BASE6     ),
        ControllerButton::Base(_)     => None,
        ControllerButton::A           => Some(input_event_codes::BTN_A         ),
        ControllerButton::B           => Some(input_event_codes::BTN_B         ),
        ControllerButton::C           => Some(input_event_codes::BTN_C         ),
        ControllerButton::X           => Some(input_event_codes::BTN_X         ),
        ControllerButton::Y           => Some(input_event_codes::BTN_Y         ),
        ControllerButton::Z           => Some(input_event_codes::BTN_Z         ),
        ControllerButton::LShoulder   => Some(input_event_codes::BTN_TL        ),
        ControllerButton::RShoulder   => Some(input_event_codes::BTN_TR        ),
        ControllerButton::LShoulder2  => Some(input_event_codes::BTN_TL2       ),
        ControllerButton::RShoulder2  => Some(input_event_codes::BTN_TR2       ),
        ControllerButton::Select      => Some(input_event_codes::BTN_SELECT    ),
        ControllerButton::Start       => Some(input_event_codes::BTN_START     ),
        ControllerButton::Mode        => Some(input_event_codes::BTN_MODE      ),
        ControllerButton::LStickClick => Some(input_event_codes::BTN_THUMBL    ),
        ControllerButton::RStickClick => Some(input_event_codes::BTN_THUMBR    ),
        ControllerButton::DpadUp      => Some(input_event_codes::BTN_DPAD_UP   ),
        ControllerButton::DpadDown    => Some(input_event_codes::BTN_DPAD_DOWN ),
        ControllerButton::DpadLeft    => Some(input_event_codes::BTN_DPAD_LEFT ),
        ControllerButton::DpadRight   => Some(input_event_codes::BTN_DPAD_RIGHT),
        ControllerButton::Num(0)      => Some(input_event_codes::BTN_0         ),
        ControllerButton::Num(1)      => Some(input_event_codes::BTN_1         ),
        ControllerButton::Num(2)      => Some(input_event_codes::BTN_2         ),
        ControllerButton::Num(3)      => Some(input_event_codes::BTN_3         ),
        ControllerButton::Num(4)      => Some(input_event_codes::BTN_4         ),
        ControllerButton::Num(5)      => Some(input_event_codes::BTN_5         ),
        ControllerButton::Num(6)      => Some(input_event_codes::BTN_6         ),
        ControllerButton::Num(7)      => Some(input_event_codes::BTN_7         ),
        ControllerButton::Num(8)      => Some(input_event_codes::BTN_8         ),
        ControllerButton::Num(9)      => Some(input_event_codes::BTN_9         ),
        ControllerButton::Num(_)      => None,
        ControllerButton::Other(other)=> Some(other as _),
    }
}

fn controller_button_from_ev_key_code(code: u16) -> ControllerButton {
    match code {
        input_event_codes::BTN_GEAR_DOWN  => ControllerButton::GearDown,
        input_event_codes::BTN_GEAR_UP    => ControllerButton::GearUp,
        input_event_codes::BTN_TRIGGER    => ControllerButton::Trigger,
        input_event_codes::BTN_THUMB      => ControllerButton::Thumb(0),
        input_event_codes::BTN_THUMB2     => ControllerButton::Thumb(1),
        input_event_codes::BTN_TOP        => ControllerButton::Top(0),
        input_event_codes::BTN_TOP2       => ControllerButton::Top(1),
        input_event_codes::BTN_PINKIE     => ControllerButton::Pinkie,
        input_event_codes::BTN_DEAD       => ControllerButton::Dead,
        input_event_codes::BTN_BASE       => ControllerButton::Base(0),
        input_event_codes::BTN_BASE2      => ControllerButton::Base(1),
        input_event_codes::BTN_BASE3      => ControllerButton::Base(2),
        input_event_codes::BTN_BASE4      => ControllerButton::Base(3),
        input_event_codes::BTN_BASE5      => ControllerButton::Base(4),
        input_event_codes::BTN_BASE6      => ControllerButton::Base(5),
        input_event_codes::BTN_A          => ControllerButton::A,
        input_event_codes::BTN_B          => ControllerButton::B,
        input_event_codes::BTN_C          => ControllerButton::C,
        input_event_codes::BTN_X          => ControllerButton::X,
        input_event_codes::BTN_Y          => ControllerButton::Y,
        input_event_codes::BTN_Z          => ControllerButton::Z,
        input_event_codes::BTN_TL         => ControllerButton::LShoulder,
        input_event_codes::BTN_TR         => ControllerButton::RShoulder,
        input_event_codes::BTN_TL2        => ControllerButton::LShoulder2,
        input_event_codes::BTN_TR2        => ControllerButton::RShoulder2,
        input_event_codes::BTN_SELECT     => ControllerButton::Select,
        input_event_codes::BTN_START      => ControllerButton::Start,
        input_event_codes::BTN_MODE       => ControllerButton::Mode,
        input_event_codes::BTN_THUMBL     => ControllerButton::LStickClick,
        input_event_codes::BTN_THUMBR     => ControllerButton::RStickClick,
        input_event_codes::BTN_DPAD_UP    => ControllerButton::DpadUp,
        input_event_codes::BTN_DPAD_DOWN  => ControllerButton::DpadDown,
        input_event_codes::BTN_DPAD_LEFT  => ControllerButton::DpadLeft,
        input_event_codes::BTN_DPAD_RIGHT => ControllerButton::DpadRight,
        input_event_codes::BTN_0          => ControllerButton::Num(0),
        input_event_codes::BTN_1          => ControllerButton::Num(1),
        input_event_codes::BTN_2          => ControllerButton::Num(2),
        input_event_codes::BTN_3          => ControllerButton::Num(3),
        input_event_codes::BTN_4          => ControllerButton::Num(4),
        input_event_codes::BTN_5          => ControllerButton::Num(5),
        input_event_codes::BTN_6          => ControllerButton::Num(6),
        input_event_codes::BTN_7          => ControllerButton::Num(7),
        input_event_codes::BTN_8          => ControllerButton::Num(8),
        input_event_codes::BTN_9          => ControllerButton::Num(9),
        other => ControllerButton::Other(other as _),
    }
}

fn controller_axis_from_ev_abs_code_common(code: u16) -> ControllerAxis {
    match code {
        input_event_codes::ABS_THROTTLE => ControllerAxis::Throttle,
        input_event_codes::ABS_RUDDER   => ControllerAxis::Rudder  ,
        input_event_codes::ABS_WHEEL    => ControllerAxis::Wheel   ,
        input_event_codes::ABS_GAS      => ControllerAxis::Gas     ,
        input_event_codes::ABS_BRAKE    => ControllerAxis::Brake   ,
        input_event_codes::ABS_HAT1X    => ControllerAxis::HatX(1) ,
        input_event_codes::ABS_HAT1Y    => ControllerAxis::HatY(1) ,
        input_event_codes::ABS_HAT2X    => ControllerAxis::HatX(2) ,
        input_event_codes::ABS_HAT2Y    => ControllerAxis::HatY(2) ,
        input_event_codes::ABS_HAT3X    => ControllerAxis::HatX(3) ,
        input_event_codes::ABS_HAT3Y    => ControllerAxis::HatY(3) ,
        input_event_codes::ABS_X        => unreachable!{},
        input_event_codes::ABS_Y        => unreachable!{},
        input_event_codes::ABS_Z        => unreachable!{},
        input_event_codes::ABS_RX       => unreachable!{},
        input_event_codes::ABS_RY       => unreachable!{},
        input_event_codes::ABS_RZ       => unreachable!{},
        input_event_codes::ABS_HAT0X    => unreachable!{},
        input_event_codes::ABS_HAT0Y    => unreachable!{},
        other => ControllerAxis::Other(other as _),
    }
}
fn controller_axis_from_ev_abs_code_for_gamepads_or_steering_wheels(code: u16) -> ControllerAxis {
    match code {
        input_event_codes::ABS_X        => ControllerAxis::LX,
        input_event_codes::ABS_Y        => ControllerAxis::LY,
        input_event_codes::ABS_Z        => ControllerAxis::LTrigger,
        input_event_codes::ABS_RX       => ControllerAxis::RX,
        input_event_codes::ABS_RY       => ControllerAxis::RY,
        input_event_codes::ABS_RZ       => ControllerAxis::RTrigger,
        input_event_codes::ABS_HAT0X    => ControllerAxis::DpadX,
        input_event_codes::ABS_HAT0Y    => ControllerAxis::DpadY,
        _ => controller_axis_from_ev_abs_code_common(code),
    }
}
fn controller_axis_from_ev_abs_code_for_joysticks(code: u16) -> ControllerAxis {
    match code {
        input_event_codes::ABS_X        => ControllerAxis::JoystickX,
        input_event_codes::ABS_Y        => ControllerAxis::JoystickY,
        input_event_codes::ABS_Z        => ControllerAxis::JoystickZ,
        input_event_codes::ABS_RX       => ControllerAxis::JoystickRotationX,
        input_event_codes::ABS_RY       => ControllerAxis::JoystickRotationY,
        input_event_codes::ABS_RZ       => ControllerAxis::JoystickRotationZ,
        input_event_codes::ABS_HAT0X    => ControllerAxis::HatX(0),
        input_event_codes::ABS_HAT0Y    => ControllerAxis::HatY(0),
        _ => controller_axis_from_ev_abs_code_common(code),
    }
}

fn controller_axis_to_ev_abs_code_for_gamepads_or_steering_wheels(axis: ControllerAxis) -> Option<u16> {
    match axis {
        ControllerAxis::Throttle          => Some(input_event_codes::ABS_THROTTLE),
        ControllerAxis::Rudder            => Some(input_event_codes::ABS_RUDDER  ),
        ControllerAxis::Wheel             => Some(input_event_codes::ABS_WHEEL   ),
        ControllerAxis::Gas               => Some(input_event_codes::ABS_GAS     ),
        ControllerAxis::Brake             => Some(input_event_codes::ABS_BRAKE   ),
        ControllerAxis::LX                => Some(input_event_codes::ABS_X       ),
        ControllerAxis::LY                => Some(input_event_codes::ABS_Y       ),
        ControllerAxis::RX                => Some(input_event_codes::ABS_RX      ),
        ControllerAxis::RY                => Some(input_event_codes::ABS_RY      ),
        ControllerAxis::LTrigger          => Some(input_event_codes::ABS_Z       ),
        ControllerAxis::RTrigger          => Some(input_event_codes::ABS_RZ      ),
        ControllerAxis::DpadX             => Some(input_event_codes::ABS_HAT0X   ),
        ControllerAxis::DpadY             => Some(input_event_codes::ABS_HAT0Y   ),
        ControllerAxis::HatX(1)           => Some(input_event_codes::ABS_HAT1X   ),
        ControllerAxis::HatY(1)           => Some(input_event_codes::ABS_HAT1Y   ),
        ControllerAxis::HatX(2)           => Some(input_event_codes::ABS_HAT2X   ),
        ControllerAxis::HatY(2)           => Some(input_event_codes::ABS_HAT2Y   ),
        ControllerAxis::HatX(3)           => Some(input_event_codes::ABS_HAT3X   ),
        ControllerAxis::HatY(3)           => Some(input_event_codes::ABS_HAT3Y   ),
        ControllerAxis::HatX(_)           => None,
        ControllerAxis::HatY(_)           => None,
        ControllerAxis::Other(other)      => Some(other as _),
        ControllerAxis::JoystickX         => None,
        ControllerAxis::JoystickY         => None,
        ControllerAxis::JoystickZ         => None,
        ControllerAxis::JoystickRotationX => None,
        ControllerAxis::JoystickRotationY => None,
        ControllerAxis::JoystickRotationZ => None,
    }
}

fn controller_axis_to_ev_abs_code_for_joysticks(axis: ControllerAxis) -> Option<u16> {
    match axis {
        ControllerAxis::Throttle          => Some(input_event_codes::ABS_THROTTLE),
        ControllerAxis::Rudder            => Some(input_event_codes::ABS_RUDDER  ),
        ControllerAxis::Wheel             => Some(input_event_codes::ABS_WHEEL   ),
        ControllerAxis::Gas               => Some(input_event_codes::ABS_GAS     ),
        ControllerAxis::Brake             => Some(input_event_codes::ABS_BRAKE   ),
        ControllerAxis::JoystickX         => Some(input_event_codes::ABS_X       ),
        ControllerAxis::JoystickY         => Some(input_event_codes::ABS_Y       ),
        ControllerAxis::JoystickZ         => Some(input_event_codes::ABS_Z       ),
        ControllerAxis::JoystickRotationX => Some(input_event_codes::ABS_RX      ),
        ControllerAxis::JoystickRotationY => Some(input_event_codes::ABS_RY      ),
        ControllerAxis::JoystickRotationZ => Some(input_event_codes::ABS_RZ      ),
        ControllerAxis::HatX(0)           => Some(input_event_codes::ABS_HAT0X   ),
        ControllerAxis::HatY(0)           => Some(input_event_codes::ABS_HAT0Y   ),
        ControllerAxis::HatX(1)           => Some(input_event_codes::ABS_HAT1X   ),
        ControllerAxis::HatY(1)           => Some(input_event_codes::ABS_HAT1Y   ),
        ControllerAxis::HatX(2)           => Some(input_event_codes::ABS_HAT2X   ),
        ControllerAxis::HatY(2)           => Some(input_event_codes::ABS_HAT2Y   ),
        ControllerAxis::HatX(3)           => Some(input_event_codes::ABS_HAT3X   ),
        ControllerAxis::HatY(3)           => Some(input_event_codes::ABS_HAT3Y   ),
        ControllerAxis::HatX(_)           => None,
        ControllerAxis::HatY(_)           => None,
        ControllerAxis::Other(other)      => Some(other as _),
        ControllerAxis::LX                => None,
        ControllerAxis::LY                => None,
        ControllerAxis::DpadX             => None,
        ControllerAxis::DpadY             => None,
        ControllerAxis::RX                => None,
        ControllerAxis::RY                => None,
        ControllerAxis::LTrigger          => None,
        ControllerAxis::RTrigger          => None,
    }
}


impl Linuxdev {
    pub fn translate_linux_input_event(&self, with_token: LinuxdevToken, ev: &linux_input::input_event) -> Option<Event> {
        let &linux_input::input_event {
            time, type_, code, value
        } = ev;
        let instant = {
            let c::timeval { tv_sec, tv_usec } = time;
            EventInstant(OsEventInstant::LinuxInputEventTimeval { tv_sec, tv_usec })
        };
        let controller = DeviceID(OsDeviceID::Linuxdev(with_token));
        match type_ {
            input_event_codes::EV_KEY => {
                let button = self.translate_ev_key(code);
                let ev = if value == 0 {
                    Event::ControllerButtonReleased { controller, instant, button }
                } else {
                    Event::ControllerButtonPressed { controller, instant, button }
                };
                Some(ev)
            },
            input_event_codes::EV_ABS => {
                Some(Event::ControllerAxisMotion {
                    controller, instant, axis: self.translate_ev_abs(code), value: value as _,
                })
            },
            input_event_codes::EV_REL => None,
            input_event_codes::EV_MSC => None,
            input_event_codes::EV_SYN => None,
            input_event_codes::EV_SW  => None,
            input_event_codes::EV_LED => None,
            input_event_codes::EV_SND => None,
            input_event_codes::EV_REP => None,
            input_event_codes::EV_FF  => None,
            input_event_codes::EV_PWR => None,
            input_event_codes::EV_FF_STATUS => None,
            _ => None,
        }
    }

    fn pump_evdev(&self, with_token: LinuxdevToken, pending_translated_events: &mut VecDeque<Event>) -> device::Result<()> {
        if let Some(evdev) = self.evdev.as_ref() {
            let mut ev: linux_input::input_event = unsafe { mem::zeroed() };
            let mut read_flag = libevdev_read_flag::LIBEVDEV_READ_FLAG_NORMAL;
            loop {
                let status = unsafe {
                    evdev::libevdev_next_event(evdev.libevdev, read_flag as _, &mut ev)
                };
                match status {
                    s if s == -c::EAGAIN || s == -c::EWOULDBLOCK => {
                        if read_flag as u32 == libevdev_read_flag::LIBEVDEV_READ_FLAG_SYNC as u32 {
                            read_flag = libevdev_read_flag::LIBEVDEV_READ_FLAG_NORMAL;
                        } else {
                            break Ok(());
                        }
                    },
                    s if s == libevdev_read_status::LIBEVDEV_READ_STATUS_SUCCESS as _ => {
                        if let Some(ev) = self.translate_linux_input_event(with_token, &ev) {
                            pending_translated_events.push_back(ev);
                        }
                    },
                    s if s == libevdev_read_status::LIBEVDEV_READ_STATUS_SYNC as _ => {
                        read_flag = libevdev_read_flag::LIBEVDEV_READ_FLAG_SYNC;
                        if let Some(ev) = self.translate_linux_input_event(with_token, &ev) {
                            pending_translated_events.push_back(ev);
                        }
                    },
                    s if s == -c::ENODEV => break device::disconnected(),
                    other => break device::failed(format!("Controller {}: libevdev_next_event() returned -{}", self.display(), Errno::from_i32(-other))),
                };
            }
        } else {
            Ok(())
        }
    }

    fn controller_state(&self) -> device::Result<OsControllerState> {
        let state = OsControllerState {
            buttons: self.evdev_all_controller_buttons_state(),
            axes: self.evdev_all_controller_axes_state(),
        };
        Ok(state)
    }
    fn controller_button_state(&self, button: ControllerButton) -> device::Result<ButtonState> {
        match self.evdev_controller_button_state(button) {
            Some(state) => Ok(state),
            None => device::not_supported_by_device_unexplained(),
        }
    }
    fn controller_axis_state(&self, axis: ControllerAxis) -> device::Result<f64> {
        match self.evdev_controller_axis_state(axis) {
            Some(state) => Ok(state),
            None => device::not_supported_by_device_unexplained(),
        }
    }


    //
    // For this part, see
    // https://www.kernel.org/doc/Documentation/input/ff.txt
    //

    fn controller_set_vibration(&self, vibration: &VibrationState) -> device::Result<()> {
        if self.evdev.is_none() {
            return device::not_supported_by_device_unexplained();
        }
        assert!(self.fd.is_some());
        let evdev = self.evdev.as_ref().unwrap();

        if !self.fd_has_write_access {
            return device::not_supported_by_device("Device file could not be opened for write access");
        }
        if evdev.props.max_simultaneous_ff_effects < 1 {
            return device::not_supported_by_device("Device does not support playing at least one force feedback effect");
        }
        if vibration.is_zero() && evdev.rumble_ff_id.get() == -1 {
            return Ok(());
        }

        let mut ff = vibration.to_ff_effect();
        let number_of_times_to_play = ::std::i32::MAX * vibration.is_zero() as i32;

        match evdev.rumble_ff_id.get() {
            -1 => {
                assert!(!vibration.is_zero());
                assert_eq!(ff.id, -1);
                self.register_ff_effect(&mut ff)?;
                assert_ne!(ff.id, -1);
                evdev.rumble_ff_id.set(ff.id);
                // Full power!! we want the vibration to reflect the full capabilities
                // of the device; the VibrationState is already a percentage of the
                // amount of vibration wanted by the user, so let's not be slowed
                // down by some default gain value that is lower than 100%.
                let max_ff_gain = 0xffff_i32;
                let status = self.set_ff_gain(max_ff_gain);
                match status {
                    Err(device::Error::DeviceDisconnected(_)) => status?,
                    Err(e) => warn!("Controller {}: Could not set FF_GAIN to {:x}: write() returned {}", self.display(), max_ff_gain, e),
                    Ok(()) => (),
                };
                self.write_ff_event(ff.id as u16, number_of_times_to_play)
            },
            id => {
                ff.id = id;
                self.register_ff_effect(&mut ff)?;
                self.write_ff_event(id as u16, number_of_times_to_play)
            },
        }
    }
    // The FF_GAIN property defines a factor for the strength of force-feedback
    // effects. It ranges from 0% to 100% (0 to 0xffff).
    // The default value is not specified, but I've seen it set to something
    // like 75%.
    fn set_ff_gain(&self, gain: i32) -> device::Result<()> {
        assert!(gain >= 0);
        assert!(gain <= 0xffff);
        self.write_ff_event(ff::FF_GAIN as u16, gain)
    }
    fn register_ff_effect(&self, ff: &mut linux_input::ff_effect) -> device::Result<()> {
        let fd = self.fd.unwrap();
        assert!(self.fd_has_write_access);

        // Upload the effect. If its id is set to -1, the kernel or driver changes it to a valid value.
        let status = unsafe {
            ev_ioctl::register_ff_effect(fd, ff)
        };
        match status {
            Err(nix::Error::Sys(Errno::ENODEV)) => return device::disconnected(),
            Err(e) => return device::not_supported_by_device(format!("Controller {}: could not register force feedback effect: ioctl() generated {}", self.display(), e)),
            Ok(_) => (),
        }
        if ff.id == -1 {
            return device::not_supported_by_device(format!("Controller {}: force feedback effect ID was not set by the kernel or driver; We have no way to reference it later!", self.display()));
        }
        Ok(())
    }
    fn write_ff_event(&self, code: u16, value: i32) -> device::Result<()> {
        let fd = self.fd.unwrap();
        assert!(self.fd_has_write_access);

        let ev = linux_input::input_event {
            type_: input_event_codes::EV_FF,
            code,
            value,
            time: unsafe { mem::zeroed() },
        };
        loop {
            let size = mem::size_of_val(&ev);
            let nwritten = unsafe {
                c::write(fd, &ev as *const _ as _, size)
            };
            if nwritten == -1 {
                #[allow(unreachable_patterns)] // EAGAIN and EWOULDBLOCK are the same on Linux, but maybe not on all other systems. It's more of a paranoid "what if" matter.
                match Errno::last() {
                    Errno::EAGAIN | errno::EWOULDBLOCK => continue,
                    Errno::ENODEV => break device::disconnected(),
                    err => break device::not_supported_by_device(format!("Controller {}: could not play rumble effect: write() generated {}", self.display(), err)),
                };
            } else if nwritten != size as isize {
                // Like, this really should never happen. But let's not assert on it.
                break device::not_supported_by_device_unexplained();
            }
            break Ok(());
        }
    }
}

impl VibrationState {
    pub(self) fn to_ff_effect(&self) -> linux_input::ff_effect {
        let &Self {
            weak_magnitude, strong_magnitude,
        } = self;

        linux_input::ff_effect {
            type_: ff::FF_RUMBLE,
            id: -1,
            /*
             * Direction is encoded as follows:
             * 0 deg -> 0x0000 (down)
             * 90 deg -> 0x4000 (left)
             * 180 deg -> 0x8000 (up)
             * 270 deg -> 0xC000 (right)
             */
            direction: 0,
            trigger: linux_input::ff_trigger {
                button: 0,
                interval: 0,
            },
            replay: linux_input::ff_replay {
                // From linux/input.h:
                // "Values above 32767 ms (0x7fff) should not be used and have unspecified results."
                length: 0x7fff,
                delay: 0,
            },
            u: {
                let mut u = linux_input::ff_effect_union::default();
                unsafe {
                    *u.rumble() = linux_input::ff_rumble_effect {
                        strong_magnitude,
                        weak_magnitude,
                    };
                }
                u
            },
        }
    }
}

#[allow(dead_code)]
// These are missing from all bindings I've searched for. Geez people
mod ff {
    // Values describing the status of a force-feedback effect
    pub const FF_STATUS_STOPPED: u16 = 0x00;
    pub const FF_STATUS_PLAYING: u16 = 0x01;

    // Force feedback effect types
    pub const FF_RUMBLE  : u16 = 0x50;
    pub const FF_PERIODIC: u16 = 0x51;
    pub const FF_CONSTANT: u16 = 0x52;
    pub const FF_SPRING  : u16 = 0x53;
    pub const FF_FRICTION: u16 = 0x54;
    pub const FF_DAMPER  : u16 = 0x55;
    pub const FF_INERTIA : u16 = 0x56;
    pub const FF_RAMP    : u16 = 0x57;

    pub const FF_EFFECT_MIN: u16 = FF_RUMBLE;
    pub const FF_EFFECT_MAX: u16 = FF_RAMP;

    // Force feedback periodic effect types
    pub const FF_SQUARE   : u16 = 0x58;
    pub const FF_TRIANGLE : u16 = 0x59;
    pub const FF_SINE     : u16 = 0x5a;
    pub const FF_SAW_UP   : u16 = 0x5b;
    pub const FF_SAW_DOWN : u16 = 0x5c;
    pub const FF_CUSTOM   : u16 = 0x5d;

    pub const FF_WAVEFORM_MIN: u16 = FF_SQUARE;
    pub const FF_WAVEFORM_MAX: u16 = FF_CUSTOM;

    // Set ff device properties
    pub const FF_GAIN      : u16 = 0x60;
    pub const FF_AUTOCENTER: u16 = 0x61;
}

#[allow(dead_code)]
mod linux_bus_id {
    pub const BUS_PCI        : i32 = 0x01;
    pub const BUS_ISAPNP     : i32 = 0x02;
    pub const BUS_USB        : i32 = 0x03;
    pub const BUS_HIL        : i32 = 0x04;
    pub const BUS_BLUETOOTH  : i32 = 0x05;
    pub const BUS_VIRTUAL    : i32 = 0x06;

    pub const BUS_ISA        : i32 = 0x10;
    pub const BUS_I8042      : i32 = 0x11;
    pub const BUS_XTKBD      : i32 = 0x12;
    pub const BUS_RS232      : i32 = 0x13;
    pub const BUS_GAMEPORT   : i32 = 0x14;
    pub const BUS_PARPORT    : i32 = 0x15;
    pub const BUS_AMIGA      : i32 = 0x16;
    pub const BUS_ADB        : i32 = 0x17;
    pub const BUS_I2C        : i32 = 0x18;
    pub const BUS_HOST       : i32 = 0x19;
    pub const BUS_GSC        : i32 = 0x1A;
    pub const BUS_ATARI      : i32 = 0x1B;
    pub const BUS_SPI        : i32 = 0x1C;
    pub const BUS_RMI        : i32 = 0x1D;
    pub const BUS_CEC        : i32 = 0x1E;
    pub const BUS_INTEL_ISHTP: i32 = 0x1F;
}


#[allow(dead_code)]
// I might have gotten some of these wrong.
mod ev_ioctl {
    use super::*;

    // #define EVIOCGVERSION		_IOR('E', 0x01, int)			/* get driver version */
    ioctl!(read get_driver_version with b'E', 0x01; c_int);
    // #define EVIOCGID		_IOR('E', 0x02, struct input_id)	/* get device ID */
    ioctl!(read get_device_id with b'E', 0x02; linux_input::input_id);
    // #define EVIOCGREP		_IOR('E', 0x03, unsigned int[2])	/* get repeat settings */
    ioctl!(read_buf get_repeat_settings with b'E', 0x03; c_uint);
    // #define EVIOCSREP		_IOW('E', 0x03, unsigned int[2])	/* set repeat settings */
    ioctl!(write_buf set_repeat_settings with b'E', 0x03; c_uint);

    // #define EVIOCGKEYCODE		_IOR('E', 0x04, unsigned int[2])        /* get keycode */
    ioctl!(read_buf get_keycode with b'E', 0x04; c_uint);
    // #define EVIOCGKEYCODE_V2	_IOR('E', 0x04, struct input_keymap_entry)
    ioctl!(read get_keycode_v2 with b'E', 0x04; linux_input::input_keymap_entry);
    // #define EVIOCSKEYCODE		_IOW('E', 0x04, unsigned int[2])        /* set keycode */
    ioctl!(write_buf set_keycode with b'E', 0x04; c_uint);
    // #define EVIOCSKEYCODE_V2	_IOW('E', 0x04, struct input_keymap_entry)
    ioctl!(write_ptr set_keycode_v2 with b'E', 0x04; linux_input::input_keymap_entry);

    // #define EVIOCGNAME(len)		_IOC(_IOC_READ, 'E', 0x06, len)		/* get device name */
    ioctl!(read_buf get_name with b'E', 0x06; c_char);
    // #define EVIOCGPHYS(len)		_IOC(_IOC_READ, 'E', 0x07, len)		/* get physical location */
    ioctl!(read_buf get_physical_location with b'E', 0x07; c_char);
    // #define EVIOCGUNIQ(len)		_IOC(_IOC_READ, 'E', 0x08, len)		/* get unique identifier */
    ioctl!(read_buf get_unique_identifier with b'E', 0x08; c_char);
    // #define EVIOCGPROP(len)		_IOC(_IOC_READ, 'E', 0x09, len)		/* get device properties */
    ioctl!(read_buf get_properties with b'E', 0x09; c_char);

/*
#define EVIOCGMTSLOTS(len)	_IOC(_IOC_READ, 'E', 0x0a, len)

#define EVIOCGKEY(len)		_IOC(_IOC_READ, 'E', 0x18, len)		/* get global key state */
#define EVIOCGLED(len)		_IOC(_IOC_READ, 'E', 0x19, len)		/* get all LEDs */
#define EVIOCGSND(len)		_IOC(_IOC_READ, 'E', 0x1a, len)		/* get all sounds status */
#define EVIOCGSW(len)		_IOC(_IOC_READ, 'E', 0x1b, len)		/* get all switch states */

#define EVIOCGBIT(ev,len)	_IOC(_IOC_READ, 'E', 0x20 + (ev), len)	/* get event bits */
#define EVIOCGABS(abs)		_IOR('E', 0x40 + (abs), struct input_absinfo)	/* get abs value/limits */
#define EVIOCSABS(abs)		_IOW('E', 0xc0 + (abs), struct input_absinfo)	/* set abs value/limits */
*/

    // #define EVIOCSFF		_IOW('E', 0x80, struct ff_effect)	/* send a force effect to a force feedback device */
    ioctl!(write_ptr register_ff_effect with b'E', 0x80; linux_input::ff_effect);
    // #define EVIOCRMFF		_IOW('E', 0x81, int)			/* Erase a force effect */
    ioctl!(write_int unregister_ff_effect with b'E', 0x81);
    // #define EVIOCGEFFECTS		_IOR('E', 0x84, int)			/* Report number of effects playable at the same time */
    ioctl!(read max_simultaneous_ff_effects with b'E', 0x84; c_int);

/*
#define EVIOCGRAB		_IOW('E', 0x90, int)			/* Grab/Release device */
#define EVIOCREVOKE		_IOW('E', 0x91, int)			/* Revoke device access */

#define EVIOCGMASK		_IOR('E', 0x92, struct input_mask)	/* Get event-masks */

#define EVIOCSMASK		_IOW('E', 0x93, struct input_mask)	/* Set event-masks */

#define EVIOCSCLOCKID		_IOW('E', 0xa0, int)			/* Set clockid to be used for timestamps */
*/
}

