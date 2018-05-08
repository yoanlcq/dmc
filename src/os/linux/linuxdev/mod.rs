// Very interesting doc:
// https://www.kernel.org/doc/html/v4.12/input/gamepad.html

extern crate libevdev_sys;
extern crate libudev_sys;
extern crate libc as c;

use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::CStr;
use std::ptr;
use std::mem;
use std::cell::{Cell, RefCell};
use std::time::Duration;
use event::{Event, EventInstant};
use os::{OsEventInstant, OsHidID};
use time_utils;
use hid::{self, HidID, HidInfo, ControllerInfo, ControllerAxis, ControllerState, ControllerButton, ButtonState, Bus, RumbleEffect, AxisInfo};

use self::c::{c_int, c_uint, c_char};

use nix::errno::{self, Errno};

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
    controllers: RefCell<HashMap<LinuxdevToken, Linuxdev>>,
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
    /// The last Force-Feedback ID or -1.
    last_ff_id: Cell<i16>,
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
    props: EvdevProps,
    buttons: HashSet<ControllerButton>,
    axes: HashMap<ControllerAxis, AxisInfo>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct EvdevProps {
    name: String,
    id_bustype: c_int,
    id_product: u16,
    id_vendor: u16,
    is_a_steering_wheel: bool,
    is_a_gamepad: bool,
    is_a_joystick: bool,
    repeat: Option<EvdevRepeat>,
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
            controllers: _,
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
            ref last_ff_id,
        } = self;
        unsafe {
            if owns_udev_device {
                libudev_sys::udev_device_unref(udev_device);
            }
            if let Some(evdev) = evdev.as_ref() {
                evdev::libevdev_free(evdev.libevdev);
            }
            if let Some(fd) = fd {
                if last_ff_id.get() != -1 {
                    let res = ev_ioctl::unregister_ff_effect(fd, last_ff_id.get() as _);
                    if let Err(e) = res {
                        error!("Controller {}: failed to unregister the last playing force feedback effect while dropping it! (ioctl() generated {})", udev_props.display(), e);
                    }
                }
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
            let mut controllers = HashMap::with_capacity(32);

            for entry in udev_enumerate::scan_devices_iter(udev_enumerate) {
                let _entry_value = libudev_sys::udev_list_entry_get_value(entry);
                let devname = libudev_sys::udev_list_entry_get_name(entry);
                assert!(!devname.is_null());
                let udev_device = libudev_sys::udev_device_new_from_syspath(udev, devname);
                let dev = Linuxdev::from_udev_device(FromUdevDevice {
                    udev_device, 
                    owns_udev_device: true,
                    try_open_fd_if_is_a_controller: true,
                });
                if dev.is_a_controller() {
                    let token = token_generator.next_token();
                    dev.pump_evdev(token, &mut pending_translated_events);
                    controllers.insert(token, dev);
                }
            }

            Self {
                udev, udev_monitor, udev_enumerate,
                controllers: RefCell::new(controllers),
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
        if let Some(&Event::HidDisconnected { hid: HidID(OsHidID::Linuxdev(token)), .. }) = ev.as_ref() {

            self.controllers.borrow_mut().remove(&token);
        }
        ev
    }
    fn pump_events(&self) {
        for (token, dev) in self.controllers.borrow().iter() {
            dev.pump_evdev(*token, &mut self.pending_translated_events.borrow_mut());
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
        if !dev.is_a_controller() {
            return;
        }
        let token = self.token_generator.borrow_mut().next_token();
        let hid_connected_event = Event::HidConnected {
            hid: HidID(OsHidID::Linuxdev(token)),
            instant: dev.plug_instant(),
        };
        let mut queue = self.pending_translated_events.borrow_mut();
        queue.push_back(hid_connected_event);
        dev.pump_evdev(token, &mut queue);
        self.controllers.borrow_mut().insert(token, dev);
    }
    fn on_udev_device_removed(&self, udev_device: *mut libudev_sys::udev_device) {
        // Reverse lookup
        let token = self.controllers.borrow().iter().filter_map(|(token, dev)| {
            if dev.udev_device == udev_device { Some(*token) } else { None }
        }).next();

        if token.is_none() {
            return; // It's fine; the udev_device is not necessarily a controller!
        }
        // NOTE: Don't remove the device from our list, yet !
        // Wait until the HidDisconnected event is reported to the user to do it.
        // See self.poll_next_event()
        let token = token.unwrap();
        let hid_disconnected_event = Event::HidDisconnected {
            hid: HidID(OsHidID::Linuxdev(token)),
            instant: self.controllers.borrow()[&token].instant_now(), // Looks like it's the closest we can get... ._.
        };
        self.pending_translated_events.borrow_mut().push_back(hid_disconnected_event);
    }
    pub fn controllers(&self) -> hid::Result<Vec<HidID>> {
        Ok(self.controllers.borrow().keys().map(|token| HidID(OsHidID::Linuxdev(*token))).collect())
    }
    pub fn controller_info(&self, token: LinuxdevToken) -> hid::Result<HidInfo> {
        unimplemented!{}
    }
    pub fn ping_controller(&self, token: LinuxdevToken) -> hid::Result<()> {
        unimplemented!{}
    }
    pub fn controller_state(&self, controller: HidID) -> hid::Result<ControllerState> {
        self.with_controller(controller, |dev| dev.controller_state().map(ControllerState))
    }
    pub fn controller_button_state(&self, controller: HidID, button: ControllerButton) -> hid::Result<ButtonState> {
        self.with_controller(controller, |dev| dev.controller_button_state(button))
    }
    pub fn controller_axis_state(&self, controller: HidID, axis: ControllerAxis) -> hid::Result<f64> {
        self.with_controller(controller, |dev| dev.controller_axis_state(axis))
    }
    pub fn controller_play_rumble_effect(&self, controller: HidID, effect: &RumbleEffect) -> hid::Result<()> {
        self.with_controller(controller, |dev| dev.play_rumble_effect(effect))
    }
    fn with_controller<T, F: FnMut(&Linuxdev) -> hid::Result<T>>(&self, controller: HidID, mut f: F) -> hid::Result<T> {
        if let OsHidID::Linuxdev(token) = controller.0 {
            if let Some(dev) = self.controllers.borrow().get(&token) {
                debug_assert!(dev.is_a_controller());
                f(dev)
            } else {
                hid::disconnected()    
            }
        } else {
            hid::not_supported_by_device(format!("This device does not refer to a controller"))
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


#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LinuxdevDisplay<'a> {
    name: &'a str,
}

impl<'a> ::std::fmt::Display for LinuxdevDisplay<'a> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
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

        let devnode = cstr_or_none(libudev_sys::udev_device_get_devnode(udev_device)).unwrap();

        let event_api = {
            let devnode = devnode.to_str().unwrap();
            let last_slash = devnode.rfind('/').unwrap();
            if devnode[last_slash..].starts_with("/event") {
                Some(LinuxEventAPI::Evdev)
            } else if devnode[last_slash..].starts_with("/js") {
                Some(LinuxEventAPI::Joydev)
            } else {
                None
            }
        };

        let (fd, fd_has_write_access) = {
            // Don't even try if it's not a controller. All other device kinds are "owned" by the X
            // server so we're normally not allowed to open them.
            if !try_open_fd_if_is_a_controller || !udev_props.is_a_controller() {
                (None, false)
            } else {
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
            last_ff_id: Cell::new(-1),
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
        let props = EvdevProps {
            id_bustype: evdev::libevdev_get_id_bustype(libevdev),
            id_product: evdev::libevdev_get_id_product(libevdev) as _,
            id_vendor : evdev::libevdev_get_id_vendor(libevdev) as _,
            name      : remove_quotes_if_any(CStr::from_ptr(evdev::libevdev_get_name(libevdev)).to_string_lossy().into_owned()),
            is_a_steering_wheel: {
                let has_abs_wheel = 0 != evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_ABS as _, input_event_codes::ABS_WHEEL as _);
                let has_btn_wheel = 0 != evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_WHEEL as _);
                has_abs_wheel || has_btn_wheel
            },
            is_a_gamepad : 0 != evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_GAMEPAD as _),
            is_a_joystick: 0 != evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_JOYSTICK as _),
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
        };

        Self {
            libevdev, props,
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


macro_rules! event_mapping {
    ($($EV_TYPE:ident => $(|$arg:ident|)* {$($EV_CODE:ident => $expr:expr,)*},)+) => {
        $(event_mapping!{@ $EV_TYPE => $(|$arg|)* {$($EV_CODE => $expr,)*}})+
    };
    (@ EV_KEY => {$($EV_CODE:ident => $expr:expr,)+}) => {
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        enum TranslatedEvKey {
            Button(ControllerButton),
            Dpad(ControllerAxis, i32),
        }
        impl From<ControllerButton> for TranslatedEvKey {
            fn from(b: ControllerButton) -> Self {
                TranslatedEvKey::Button(b)
            }
        }
        impl From<(ControllerAxis, i32)> for TranslatedEvKey {
            fn from(t: (ControllerAxis, i32)) -> Self {
                TranslatedEvKey::Dpad(t.0, t.1)
            }
        }
        impl Linuxdev {
            pub fn translate_ev_key(&self, code: u16) -> TranslatedEvKey {
                match code {
                    $(input_event_codes::$EV_CODE => TranslatedEvKey::from($expr),)+
                    other => TranslatedEvKey::Button(ControllerButton::Other(other as _)),
                }
            }
            pub fn untranslate_ev_key(&self, button: TranslatedEvKey) -> u16 {
                match button {
                    $(button if button == TranslatedEvKey::from($expr) => input_event_codes::$EV_CODE,)+
                    TranslatedEvKey::Button(ControllerButton::Other(other)) => other as _,
                    _ => unreachable!{},
                }
            }
            pub fn evdev_all_controller_buttons_state(&self) -> HashMap<ControllerButton, ButtonState> {
                let all_translated = &[$(TranslatedEvKey::from($expr),)+];
                let mut all_buttons = HashMap::with_capacity(all_translated.len());
                for translated in all_translated.iter() {
                    match *translated {
                        TranslatedEvKey::Dpad(_, _) => (), // Nothing to do
                        TranslatedEvKey::Button(button) => {
                            if let Some(state) = self.evdev_controller_button_state(button) {
                                all_buttons.insert(button, state);
                            }
                        },
                    };
                }
                all_buttons
            }
            pub fn evdev_controller_button_state(&self, button: ControllerButton) -> Option<ButtonState> {
                let mut value = 0;
                let status = unsafe {
                    let libevdev = self.evdev.as_ref().unwrap().libevdev;
                    let type_ = input_event_codes::EV_KEY;
                    let code = self.untranslate_ev_key(TranslatedEvKey::Button(button));
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
            pub fn evdev_refresh_all_controller_buttons_support(&mut self) {
                let buttons = self.evdev_all_controller_buttons_support();
                self.evdev.as_mut().unwrap().buttons = buttons;
            }
            pub fn evdev_all_controller_buttons_support(&self) -> HashSet<ControllerButton> {
                let libevdev = self.evdev.as_ref().unwrap().libevdev;

                let all_codes = &[$(input_event_codes::$EV_CODE,)+];
                let all_translated = &[$(TranslatedEvKey::from($expr),)+];
                assert_eq!(all_codes.len(), all_translated.len());

                let mut all_buttons = HashSet::with_capacity(all_translated.len());

                for (code, translated) in all_codes.iter().zip(all_translated.iter()) {
                    match *translated {
                        TranslatedEvKey::Dpad(_, _) => (), // Nothing to do
                        TranslatedEvKey::Button(button) => {
                            let has_it = 0 != unsafe {
                                evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, *code as _)
                            };
                            if has_it {
                                all_buttons.insert(button);
                            }
                        },
                    };
                }
                all_buttons
            }
        }
    };
    (@ EV_ABS => |$device:ident| {$($EV_CODE:ident => $expr:expr,)+}) => {
        impl Linuxdev {
            pub fn translate_ev_abs(&self, code: u16) -> ControllerAxis {
                let $device = self;
                match code {
                    $(input_event_codes::$EV_CODE => $expr,)+
                    other => ControllerAxis::Other(other as _),
                }
            }
            pub fn untranslate_ev_abs(&self, axis: ControllerAxis) -> u16 {
                let $device = self;
                match axis {
                    $(axis if axis == ($expr) => input_event_codes::$EV_CODE,)+
                    ControllerAxis::Other(other) => other as _,
                    _ => unreachable!{},
                }
            }
            pub fn evdev_all_controller_axes_state(&self) -> HashMap<ControllerAxis, f64> {
                let $device = self;
                let all_translated = &[$($expr,)+];
                let mut all_axes = HashMap::with_capacity(all_translated.len());
                for translated in all_translated.iter() {
                    if let Some(state) = self.evdev_controller_axis_state(*translated) {
                        all_axes.insert(*translated, state);
                    }
                }
                all_axes
            }
            pub fn evdev_controller_axis_state(&self, axis: ControllerAxis) -> Option<f64> {
                let libevdev = self.evdev.as_ref().unwrap().libevdev;
                {
                    let mut value = 0;
                    let status = unsafe {
                        let type_ = input_event_codes::EV_ABS;
                        let code = self.untranslate_ev_abs(axis);
                        evdev::libevdev_fetch_event_value(libevdev, type_ as _, code as _, &mut value)
                    };
                    if status != 0 {
                        return Some(value as f64);
                    };
                }
                // Special case for D-pad
                match axis {
                    ControllerAxis::DpadX => {
                        let mut dpad_lt_value = 0;
                        let mut dpad_rt_value = 0;
                        let has_dpad_lt = 0 != unsafe { evdev::libevdev_fetch_event_value(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_DPAD_LEFT as _, &mut dpad_lt_value) };
                        let has_dpad_rt = 0 != unsafe { evdev::libevdev_fetch_event_value(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_DPAD_RIGHT as _, &mut dpad_rt_value) };
                        if !has_dpad_lt && !has_dpad_rt {
                            None
                        } else if has_dpad_lt && !has_dpad_rt {
                            Some(-dpad_lt_value as f64)
                        } else if !has_dpad_lt && has_dpad_rt {
                            Some(dpad_rt_value as f64)
                        } else {
                            Some((dpad_rt_value - dpad_lt_value) as f64)
                        }
                    },
                    ControllerAxis::DpadY => {
                        let mut dpad_up_value = 0;
                        let mut dpad_dn_value = 0;
                        let has_dpad_up = 0 != unsafe { evdev::libevdev_fetch_event_value(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_DPAD_UP as _, &mut dpad_up_value) };
                        let has_dpad_dn = 0 != unsafe { evdev::libevdev_fetch_event_value(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_DPAD_DOWN as _, &mut dpad_dn_value) };
                        if !has_dpad_up && !has_dpad_dn {
                            None
                        } else if has_dpad_up && !has_dpad_dn {
                            Some(-dpad_up_value as f64)
                        } else if !has_dpad_up && has_dpad_dn {
                            Some(dpad_dn_value as f64)
                        } else {
                            Some((dpad_dn_value - dpad_up_value) as f64)
                        }
                    },
                    _ => None,
                }
            }
            pub fn evdev_refresh_all_controller_axes_support(&mut self) {
                let axes = self.evdev_all_controller_axes_support();
                self.evdev.as_mut().unwrap().axes = axes;
            }
            pub fn evdev_all_controller_axes_support(&self) -> HashMap<ControllerAxis, AxisInfo> {
                let $device = self;
                let libevdev = self.evdev.as_ref().unwrap().libevdev;

                let all_codes = &[$(input_event_codes::$EV_CODE,)+];
                let all_translated = &[$($expr,)+];
                assert_eq!(all_codes.len(), all_translated.len());

                let mut all_axes = HashMap::with_capacity(all_translated.len());

                for (code, translated) in all_codes.iter().zip(all_translated.iter()) {
                    let has_it = 0 != unsafe {
                        evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_ABS as _, *code as _)
                    };
                    if has_it {
                        let absinfo = unsafe {
                            evdev::libevdev_get_abs_info(libevdev, *code as _)
                        };
                        if !absinfo.is_null() {
                            all_axes.insert(*translated, axis_info_from_linux_absinfo(unsafe { &*absinfo }));
                        }
                    }
                }

                // Special case for D-pad, which may be reported as a button, but we provide it as an axis
                let has_dpad_up = 0 != unsafe { evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_DPAD_UP as _) };
                let has_dpad_dn = 0 != unsafe { evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_DPAD_DOWN as _) };
                let has_dpad_y = has_dpad_up || has_dpad_dn;
                let has_dpad_lt = 0 != unsafe { evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_DPAD_LEFT as _) };
                let has_dpad_rt = 0 != unsafe { evdev::libevdev_has_event_code(libevdev, input_event_codes::EV_KEY as _, input_event_codes::BTN_DPAD_RIGHT as _) };
                let has_dpad_x = has_dpad_lt || has_dpad_rt;

                let dpad_axis_info = AxisInfo {
                    range: -1. .. 1.,
                    dead_zone: None,
                    resolution: 1.,
                    fuzz: 0.,
                };
                if has_dpad_x {
                    all_axes.insert(ControllerAxis::DpadX, dpad_axis_info.clone());
                }
                if has_dpad_y {
                    all_axes.insert(ControllerAxis::DpadY, dpad_axis_info.clone());
                }
                all_axes
            }
        }
    };
    (@ EV_REL       => {$($EV_CODE:ident => $expr:expr,)+}) => {};
    (@ EV_MSC       => {$($EV_CODE:ident => $expr:expr,)+}) => {};
    (@ EV_SYN       => {$($EV_CODE:ident => $expr:expr,)*}) => {};
    (@ EV_SW        => {$($EV_CODE:ident => $expr:expr,)*}) => {};
    (@ EV_LED       => {$($EV_CODE:ident => $expr:expr,)*}) => {};
    (@ EV_SND       => {$($EV_CODE:ident => $expr:expr,)*}) => {};
    (@ EV_REP       => {$($EV_CODE:ident => $expr:expr,)*}) => {};
    (@ EV_FF        => {$($EV_CODE:ident => $expr:expr,)*}) => {};
    (@ EV_PWR       => {$($EV_CODE:ident => $expr:expr,)*}) => {};
    (@ EV_FF_STATUS => {$($EV_CODE:ident => $expr:expr,)*}) => {};
}

event_mapping!{
    EV_KEY => {
        // Wheels
        BTN_GEAR_DOWN => ControllerButton::GearDown,
        BTN_GEAR_UP => ControllerButton::GearUp,

        // Joysticks
        BTN_TRIGGER => ControllerButton::Trigger,
        BTN_THUMB => ControllerButton::Thumb(0),
        BTN_THUMB2 => ControllerButton::Thumb(1),
        BTN_TOP => ControllerButton::Top(0),
        BTN_TOP2 => ControllerButton::Top(1),
        BTN_PINKIE => ControllerButton::Pinkie,
        BTN_DEAD => ControllerButton::Dead,
        BTN_BASE => ControllerButton::Base(0),
        BTN_BASE2 => ControllerButton::Base(1),
        BTN_BASE3 => ControllerButton::Base(2),
        BTN_BASE4 => ControllerButton::Base(3),
        BTN_BASE5 => ControllerButton::Base(4),
        BTN_BASE6 => ControllerButton::Base(5),

        // Gamepads
        BTN_A => ControllerButton::A,
        BTN_B => ControllerButton::B,
        BTN_C => ControllerButton::C,
        BTN_X => ControllerButton::X,
        BTN_Y => ControllerButton::Y,
        BTN_Z => ControllerButton::Z,
        BTN_TL => ControllerButton::LShoulder,
        BTN_TR => ControllerButton::RShoulder,
        BTN_TL2 => ControllerButton::LShoulder2,
        BTN_TR2 => ControllerButton::RShoulder2,
        BTN_SELECT => ControllerButton::Select,
        BTN_START => ControllerButton::Start,
        BTN_MODE => ControllerButton::Mode,
        BTN_THUMBL => ControllerButton::LStickClick,
        BTN_THUMBR => ControllerButton::RStickClick,
        BTN_DPAD_UP => (ControllerAxis::DpadY, -1),
        BTN_DPAD_DOWN => (ControllerAxis::DpadY, 1),
        BTN_DPAD_LEFT => (ControllerAxis::DpadX, -1),
        BTN_DPAD_RIGHT => (ControllerAxis::DpadX, 1),

        // Misc
        BTN_0 => ControllerButton::Num(0),
        BTN_1 => ControllerButton::Num(1),
        BTN_2 => ControllerButton::Num(2),
        BTN_3 => ControllerButton::Num(3),
        BTN_4 => ControllerButton::Num(4),
        BTN_5 => ControllerButton::Num(5),
        BTN_6 => ControllerButton::Num(6),
        BTN_7 => ControllerButton::Num(7),
        BTN_8 => ControllerButton::Num(8),
        BTN_9 => ControllerButton::Num(9),
    },
    // We don't handle these; they are relevant for e.g mice, but not controllers.
    // Or are they?
    EV_REL => {
        REL_X    => None,
        REL_Y    => None,
        REL_Z    => None,
        REL_RX   => None,
        REL_RY   => None,
        REL_RZ   => None,
    },
    EV_ABS => |device| {
        ABS_X          => if device.is_a_gamepad() {
            ControllerAxis::LX
        } else {
            ControllerAxis::X
        },
        ABS_Y          => if device.is_a_gamepad() {
            ControllerAxis::LY
        } else {
            ControllerAxis::Y
        },
        ABS_Z          => if device.is_a_gamepad() {
            ControllerAxis::LTrigger
        } else {
            ControllerAxis::Z
        },
        // XXX: The doc for input_absinfo says that ABS_RX, ABS_RY and ABS_RZ are
        // rotational axes (for joysticks). What the heck? How is this any different
        // from the joystick's "position" ?
        // Also these definitely map to my gamepad's right stick.
        ABS_RX         => ControllerAxis::RX,
        ABS_RY         => ControllerAxis::RY,
        ABS_RZ         => if device.is_a_gamepad() {
            ControllerAxis::RTrigger
        } else {
            ControllerAxis::RZ
        },
        ABS_THROTTLE   => ControllerAxis::Throttle,
        ABS_RUDDER     => ControllerAxis::Rudder,
        ABS_WHEEL      => ControllerAxis::Wheel,
        ABS_GAS        => ControllerAxis::Gas,
        ABS_BRAKE      => ControllerAxis::Brake,
        ABS_HAT0X      => if device.is_a_gamepad() {
            ControllerAxis::DpadX
        } else {
            ControllerAxis::HatX(0)
        },
        ABS_HAT0Y      => if device.is_a_gamepad() {
            ControllerAxis::DpadY
        } else {
            ControllerAxis::HatY(0)
        },
        ABS_HAT1X      => ControllerAxis::HatX(1),
        ABS_HAT1Y      => ControllerAxis::HatY(1),
        ABS_HAT2X      => ControllerAxis::HatX(2),
        ABS_HAT2Y      => ControllerAxis::HatY(2),
        ABS_HAT3X      => ControllerAxis::HatX(3),
        ABS_HAT3Y      => ControllerAxis::HatY(3),
    },
    EV_MSC => {
        MSC_TIMESTAMP => {},
    },
    EV_SYN => {},
    EV_SW  => {},
    EV_LED => {},
    EV_SND => {},
    EV_REP => {},
    EV_FF  => {},
    EV_PWR => {},
    EV_FF_STATUS => {},
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
        let controller = HidID(OsHidID::Linuxdev(with_token));
        match type_ {
            input_event_codes::EV_KEY => {
                match self.translate_ev_key(code) {
                    TranslatedEvKey::Button(button) => Some(if value == 0 {
                        Event::ControllerButtonReleased { controller, instant, button }
                    } else {
                        Event::ControllerButtonPressed { controller, instant, button }
                    }),
                    TranslatedEvKey::Dpad(axis, value) => Some(Event::ControllerAxisMotion {
                        controller, instant, axis, value: value as _,
                    }),
                }
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

    fn pump_evdev(&self, with_token: LinuxdevToken, pending_translated_events: &mut VecDeque<Event>) {
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
                            break;
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
                    other => {
                        warn!("Controller {}: libevdev_next_event() returned -{}", self.display(), Errno::from_i32(-other));
                        break;
                    },
                };
            }
        }
    }

    fn controller_info(&self) -> hid::Result<OsControllerInfo> {
        let evdev = self.evdev.as_ref().unwrap();
        let info = OsControllerInfo {
            is_a_gamepad: self.is_a_gamepad(),
            is_a_joystick: self.is_a_joystick(),
            is_a_steering_wheel: self.is_a_steering_wheel(),
            buttons: evdev.buttons.clone(),
            axes: evdev.axes.clone(),
        };
        Ok(info)
    }
    fn controller_state(&self) -> hid::Result<OsControllerState> {
        let state = OsControllerState {
            buttons: self.evdev_all_controller_buttons_state(),
            axes: self.evdev_all_controller_axes_state(),
        };
        Ok(state)
    }
    fn controller_button_state(&self, button: ControllerButton) -> hid::Result<ButtonState> {
        match self.evdev_controller_button_state(button) {
            Some(state) => Ok(state),
            None => hid::not_supported_by_device_unexplained(),
        }
    }
    fn controller_axis_state(&self, axis: ControllerAxis) -> hid::Result<f64> {
        match self.evdev_controller_axis_state(axis) {
            Some(state) => Ok(state),
            None => hid::not_supported_by_device_unexplained(),
        }
    }
    fn play_rumble_effect(&self, effect: &RumbleEffect) -> hid::Result<()> {
        if self.fd.is_none() || !self.fd_has_write_access {
            return hid::not_supported_by_device("Device file could not be opened for write access");
        }
        let fd = self.fd.unwrap();


        if self.last_ff_id.get() != -1 {
            let status = unsafe {
                ev_ioctl::unregister_ff_effect(fd, self.last_ff_id.get() as _)
            };
            if let Err(e) = status {
                error!("Controller {}: could not unregister force feedback effect: ioctl() generated {}", self.display(), e);
            }
        }

        let mut ff = effect.to_ff_effect();

        // Upload the effect. This also changes its id from -1 to some valid value given by the kernel.
        let status = unsafe {
            ev_ioctl::register_ff_effect(fd, &mut ff)
        };
        if let Err(e) = status {
            return hid::not_supported_by_device(format!("Controller {}: could not unregister force feedback effect: ioctl() generated {}", self.display(), e));
        }
        if ff.id == -1 {
            return hid::not_supported_by_device(format!("Controller {}: force feedback effect was not set by the kernel; We have no way to reference it later!", self.display()));
        }
        self.last_ff_id.set(ff.id);

        let play = linux_input::input_event {
            type_: input_event_codes::EV_FF,
            code: ff.id as _,
            value: 1,
            time: unsafe { mem::zeroed() },
        };
        loop {
            let nwritten = unsafe {
                c::write(fd, &play as *const _ as _, mem::size_of_val(&play))
            };
            if nwritten == -1 {
                match Errno::last() {
                    Errno::EAGAIN | errno::EWOULDBLOCK => continue,
                    err => error!("Controller {}: could not play rumble effect: write() generated {}", self.display(), err),
                };
            }
            break;
        }
        Ok(())
    }
}

impl RumbleEffect {
    pub(self) fn to_ff_effect(&self) -> linux_input::ff_effect {
        let &Self {
            ref duration, weak_magnitude, strong_magnitude,
        } = self;

        // I don't know where else to put this function. It's fine here.
        // From linux/input.h:
        // "Values above 32767 ms (0x7fff) should not be used and have unspecified results."
        fn duration_to_safe_u16_millis(d: &Duration) -> u16 {
            let ms = time_utils::duration_to_millis(d);
            let max = 0x7fff;
            if ms > max {
                warn!("Duration for rumble effect will be clamped to {} ms (was {} ms) to prevent unspecified behaviour", max, ms);
                return max as u16;
            }
            ms as u16
        }

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
                length: duration_to_safe_u16_millis(duration),
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

