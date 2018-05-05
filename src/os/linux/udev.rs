extern crate libevdev_sys;
extern crate libudev_sys;
extern crate libc as c;

use std::ffi::CStr;
use std::ptr;
use std::mem;
use event::Event;
use hid::{self, ControllerID, ControllerInfo, ControllerAxis, ControllerState, ControllerButton, ButtonState};

use self::c::{c_int, c_uint, c_char};

use nix::errno;

use self::libevdev_sys::evdev;
use self::libevdev_sys::evdev::libevdev_read_flag;
use self::libevdev_sys::evdev::libevdev_read_status;
use self::libevdev_sys::linux_input;
use self::libevdev_sys::input_event_codes;


pub type UdevDeviceID = i32;

#[derive(Debug)]
pub struct UdevContext {
    pub udev: *mut libudev_sys::udev,
    pub monitor: *mut libudev_sys::udev_monitor,
    pub enumerate: *mut libudev_sys::udev_enumerate,
}

impl Drop for UdevContext {
    fn drop(&mut self) {
        let &mut Self {
            udev, monitor, enumerate,
        } = self;
        unsafe {
            libudev_sys::udev_enumerate_unref(enumerate);
            libudev_sys::udev_monitor_unref(monitor);
            libudev_sys::udev_unref(udev);
        }
    }
}

impl Default for UdevContext {
    fn default() -> Self {
        unsafe {
            let udev = libudev_sys::udev_new();
            assert!(!udev.is_null());

            let monitor = libudev_sys::udev_monitor_new_from_netlink(udev, b"udev\0".as_ptr() as _);
            assert!(!monitor.is_null());

            let status = libudev_sys::udev_monitor_enable_receiving(monitor);
            if status < 0 {
                error!("udev_monitor_enable_receiving() returned {}", status);
            }

            let enumerate = libudev_sys::udev_enumerate_new(udev);
            assert!(!enumerate.is_null());

            let status = libudev_sys::udev_enumerate_add_match_subsystem(enumerate, b"input\0".as_ptr() as _);
            if status < 0 {
                error!("udev_enumerate_add_match_subsystem() returned {}", status);
            }

            Self { udev, monitor, enumerate, }
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum UdevDeviceAction {
    Add,
    Remove,
    Change,
    Online,
    Offline,
}

impl UdevContext {
    fn poll_next_udev_monitor_event(&self) -> Option<Event> {
        let dev = unsafe {
            libudev_sys::udev_monitor_receive_device(self.monitor)
        };
        if dev.is_null() {
            return None;
        }
        let action = {
            let action = unsafe {
                cstr(libudev_sys::udev_device_get_action(dev)).unwrap()
            };
            match action.to_bytes_with_nul() {
                b"add\0" => UdevDeviceAction::Add,
                b"remove\0" => UdevDeviceAction::Remove,
                b"change\0" => UdevDeviceAction::Change,
                b"online\0" => UdevDeviceAction::Online,
                b"offline\0" => UdevDeviceAction::Offline,
                unknown => {
                    warn!("Unknown udev action `{:?}`", unknown);
                    unsafe {
                        libudev_sys::udev_device_unref(dev);
                    }
                    return None;
                },
            }
        };
        unsafe {
            libudev_sys::udev_device_unref(dev);
        }
        unimplemented!{}; // TODO: Generate Connected/Disconnected events.
    }
    pub fn poll_next_event(&self) -> Option<Event> {
        self.poll_next_udev_monitor_event()?;
        unimplemented!{}
    }
    pub fn controllers(&self) -> hid::Result<Vec<ControllerID>> {
        unimplemented!{}
    }
    pub fn controller_info(&self, controller: ControllerID) -> hid::Result<ControllerInfo> {
        unimplemented!{}
    }
    pub fn controller_state(&self, controller: ControllerID) -> hid::Result<ControllerState> {
        unimplemented!{}
    }
    pub fn controller_button_state(&self, controller: ControllerID, button: ControllerButton) -> hid::Result<ButtonState> {
        unimplemented!{}
    }
    pub fn controller_axis_state(&self, controller: ControllerID, axis: ControllerAxis) -> hid::Result<f64> {
        unimplemented!{}
    }
}



//
// --- UGLY, UNFINISHED BITS
//

// Very interesting:
// https://www.kernel.org/doc/html/v4.12/input/gamepad.html


unsafe fn cstr<'a>(ptr: *const c_char) -> Option<&'a CStr> {
    if ptr.is_null() {
        return None;
    }
    Some(&CStr::from_ptr(ptr))
}


struct EnumerateDevices {
    entry: *mut libudev_sys::udev_list_entry,
}

impl UdevContext {
    // This is unsafe because EnumerateDevices doesn't borrow Self.
    unsafe fn refresh_and_enumerate_devices(&self) -> EnumerateDevices {
        let status = libudev_sys::udev_enumerate_scan_devices(self.enumerate);
        if status < 0 {
            error!("udev_enumerate_scan_devices() returned {}", status);
        }
        let entry = libudev_sys::udev_enumerate_get_list_entry(self.enumerate);
        EnumerateDevices { entry }
    }
}

impl Iterator for EnumerateDevices {
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


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum LinuxDeviceBackend {
    Evdev,
    Joydev,
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct Device {
    udev: *mut libudev_sys::udev_device,
    fd: Option<c_int>,
    evdev: Option<*mut evdev::libevdev>,
}

impl Drop for Device {
    fn drop(&mut self) {
        let &mut Self {
            udev, fd, evdev,
        } = self;
        unsafe {
            libudev_sys::udev_device_unref(udev);
            if let Some(evdev) = evdev {
                evdev::libevdev_free(evdev);
            }
            if let Some(fd) = fd {
                c::close(fd);
            }
        }
    }
}

impl UdevContext {
    fn rescan_all_devices(&mut self) {
        unsafe {
            for entry in self.refresh_and_enumerate_devices() {
                self.handle_udev_list_entry(entry);
            }
        }
    }

    unsafe fn handle_udev_list_entry(&mut self, entry: *mut libudev_sys::udev_list_entry) {
        assert!(!entry.is_null());
        let _entry_value = libudev_sys::udev_list_entry_get_value(entry);
        let devname = libudev_sys::udev_list_entry_get_name(entry);
        let dev = libudev_sys::udev_device_new_from_syspath(self.udev, devname);
        self.handle_scanned_udev_device(dev);
        // libudev_sys::udev_device_unref(dev); 'dev' is taken ownership of
    }

    unsafe fn handle_scanned_udev_device(&mut self, dev: *mut libudev_sys::udev_device) {
        assert!(!dev.is_null());

        let devnode = cstr(libudev_sys::udev_device_get_devnode(dev)).unwrap();

        let backend = {
            let devnode = devnode.to_str().unwrap();
            let last_slash = devnode.rfind('/').unwrap();
            if devnode[last_slash..].starts_with("/event") {
                Some(LinuxDeviceBackend::Evdev)
            } else if devnode[last_slash..].starts_with("/js") {
                Some(LinuxDeviceBackend::Joydev)
            } else {
                None
            }
        };

        let (fd, evdev) = if let Some(backend) = backend {
            // O_RDWR for force feedback
            let fd = c::open(devnode.as_ptr(), c::O_RDWR | c::O_NONBLOCK, 0666);
            if fd == -1 {
                (None, None)
            } else {
                assert_eq!(backend, LinuxDeviceBackend::Evdev); // FIXME
                let evdev = {
                    let mut evdev = ptr::null_mut();
                    let status = evdev::libevdev_new_from_fd(fd, &mut evdev);
                    if status < 0 {
                        warn!("libevdev_new_from_fd() returned {}", status);
                        None
                    } else {
                        Some(evdev)
                    }
                };
                (Some(fd), evdev)
            }
        } else {
            (None, None)
        };

        let dev = Device {
            udev: dev, fd, evdev,
        };
        // TODO: Add to internal list and query info
    }
}


impl Device {
    fn udev_prop(&self, name: &[u8]) -> Option<&CStr> {
        assert_eq!(b'\0', *name.last().unwrap());
        unsafe {
            cstr(libudev_sys::udev_device_get_property_value(self.udev, name.as_ptr() as _))
        }
    }
    fn unimplemented(&self) {
        // --- Generic info
        let usec_since_initialized = unsafe {
            libudev_sys::udev_device_get_usec_since_initialized(self.udev)
        };
        let id_usb_driver = self.udev_prop(b"ID_USB_DRIVER\0");
        let id_bus = self.udev_prop(b"ID_BUS\0");
        let bustype = self.evdev.map(|evdev| unsafe { evdev::libevdev_get_id_bustype(evdev) });
        let id_serial = self.udev_prop(b"ID_SERIAL\0");
        let id_model = self.udev_prop(b"ID_MODEL\0"); // "Controller" ??
        let id_vendor = self.udev_prop(b"ID_VENDOR\0");
        let id_model_id_hex = self.udev_prop(b"ID_MODEL_ID\0");
        let id_vendor_id_hex = self.udev_prop(b"ID_VENDOR_ID\0");
        let product_id = self.evdev.map(|evdev| unsafe { evdev::libevdev_get_id_product(evdev) });
        let vendor_id  = self.evdev.map(|evdev| unsafe { evdev::libevdev_get_id_vendor(evdev) });


        // --- Name
        // TODO name: remove quotes if any
        let name_by_evdev = self.evdev.map(|evdev| unsafe { evdev::libevdev_get_name(evdev) });
        let name_by_udev = self.udev_prop(b"NAME\0");
        let name_of_parent_by_udev = {
            // NOTE: Linked to child device, no need to free it.
            let parent = unsafe {
                libudev_sys::udev_device_get_parent(self.udev)
            };
            if parent.is_null() {
                None
            } else {
                unsafe {
                    cstr(libudev_sys::udev_device_get_property_value(parent, b"NAME\0".as_ptr() as _))
                }
            }
        };


        // --- Identifying the device type
        // NOTE: from udev sources
        // https://github.com/systemd/systemd
        let id_input               = self.udev_prop(b"ID_INPUT\0");
        let id_input_joystick      = self.udev_prop(b"ID_INPUT_JOYSTICK\0");
        let id_input_accelerometer = self.udev_prop(b"ID_INPUT_ACCELEROMETER\0");
        let id_input_key           = self.udev_prop(b"ID_INPUT_KEY\0");
        let id_input_keyboard      = self.udev_prop(b"ID_INPUT_KEYBOARD\0");
        let id_input_mouse         = self.udev_prop(b"ID_INPUT_MOUSE\0");
        let id_input_pointingstick = self.udev_prop(b"ID_INPUT_POINTINGSTICK\0");
        let id_input_switch        = self.udev_prop(b"ID_INPUT_SWITCH\0");
        let id_input_tablet        = self.udev_prop(b"ID_INPUT_TABLET\0");
        let id_input_tablet_pad    = self.udev_prop(b"ID_INPUT_TABLET_PAD\0");
        let id_input_touchpad      = self.udev_prop(b"ID_INPUT_TOUCHPAD\0");
        let id_input_touchscreen   = self.udev_prop(b"ID_INPUT_TOUCHSCREEN\0");
        let id_input_trackball     = self.udev_prop(b"ID_INPUT_TRACKBALL\0");

        let is_a_steering_wheel = {
            let has_abs_wheel = self.evdev.map(|evdev| unsafe { evdev::libevdev_has_event_type(evdev, input_event_codes::ABS_WHEEL as _) });
            let has_btn_wheel = self.evdev.map(|evdev| unsafe { evdev::libevdev_has_event_type(evdev, input_event_codes::BTN_WHEEL as _) });
            has_abs_wheel.or(has_btn_wheel)
        };
        let is_a_gamepad  = self.evdev.map(|evdev| unsafe { evdev::libevdev_has_event_type(evdev, input_event_codes::BTN_GAMEPAD as _) });
        let is_a_joystick = self.evdev.map(|evdev| unsafe { evdev::libevdev_has_event_type(evdev, input_event_codes::BTN_JOYSTICK as _) });
        // There's also BTN_MOUSE, BTN_DIGI, etc... but the X server has ownership, not us.

        // --- Detecting controller features
        if let Some(evdev) = self.evdev {
            unsafe {
                let abs_info = evdev::libevdev_get_abs_info(evdev, input_event_codes::EV_KEY as _);
                let has_thumbl = evdev::libevdev_has_event_type(evdev, input_event_codes::BTN_THUMBL as _);
                let has_thumbl_as_key = evdev::libevdev_has_event_code(evdev, input_event_codes::BTN_THUMBL as _, input_event_codes::EV_KEY as _);
                let thumbl_value = evdev::libevdev_get_event_value(evdev, input_event_codes::BTN_THUMBL as _, input_event_codes::EV_KEY as _);
                let repeat = {
                    let (mut delay, mut period) = (0, 0);
                    let status = evdev::libevdev_get_repeat(evdev, &mut delay, &mut period);
                    if status < 0 {
                        None
                    } else {
                        Some((delay, period))
                    }
                };
            }
        }
    }
}


fn handle_linux_input_event(ev: &linux_input::input_event) {
    let &linux_input::input_event {
        time, type_, code, value
    } = ev;
    match type_ {
        input_event_codes::EV_KEY => match code {
            // Wheels
            input_event_codes::BTN_GEAR_DOWN => (),
            input_event_codes::BTN_GEAR_UP => (),

            // Joysticks
            input_event_codes::BTN_TRIGGER => (),
            input_event_codes::BTN_THUMB => (),
            input_event_codes::BTN_THUMB2 => (),
            input_event_codes::BTN_TOP => (),
            input_event_codes::BTN_TOP2 => (),
            input_event_codes::BTN_PINKIE => (),
            input_event_codes::BTN_DEAD => (),
            input_event_codes::BTN_BASE => (),
            input_event_codes::BTN_BASE2 => (),
            input_event_codes::BTN_BASE3 => (),
            input_event_codes::BTN_BASE4 => (),
            input_event_codes::BTN_BASE5 => (),
            input_event_codes::BTN_BASE6 => (),

            // Gamepads
            input_event_codes::BTN_A => (),
            input_event_codes::BTN_B => (),
            input_event_codes::BTN_C => (),
            input_event_codes::BTN_X => (),
            input_event_codes::BTN_Y => (),
            input_event_codes::BTN_Z => (),
            input_event_codes::BTN_TL => (),
            input_event_codes::BTN_TR => (),
            input_event_codes::BTN_TL2 => (),
            input_event_codes::BTN_TR2 => (),
            input_event_codes::BTN_SELECT => (),
            input_event_codes::BTN_START => (),
            input_event_codes::BTN_MODE => (),
            input_event_codes::BTN_THUMBL => (),
            input_event_codes::BTN_THUMBR => (),
            input_event_codes::BTN_DPAD_UP => (),
            input_event_codes::BTN_DPAD_DOWN => (),
            input_event_codes::BTN_DPAD_LEFT => (),
            input_event_codes::BTN_DPAD_RIGHT => (),

            // Misc
            input_event_codes::BTN_0 => (),
            input_event_codes::BTN_1 => (),
            input_event_codes::BTN_2 => (),
            input_event_codes::BTN_3 => (),
            input_event_codes::BTN_4 => (),
            input_event_codes::BTN_5 => (),
            input_event_codes::BTN_6 => (),
            input_event_codes::BTN_7 => (),
            input_event_codes::BTN_8 => (),
            input_event_codes::BTN_9 => (),
            _ => (),
        },
        input_event_codes::EV_REL => match code {
            input_event_codes::REL_X => (),
            input_event_codes::REL_Y => (),
            input_event_codes::REL_Z => (),
            input_event_codes::REL_RX => (),
            input_event_codes::REL_RY => (),
            input_event_codes::REL_RZ => (),
            input_event_codes::REL_MISC => (),
            _ => (),
        },
        // Xbox 360 reports dpad as HAT0X and HAT0Y (downwards).
        input_event_codes::EV_ABS => match code {
            input_event_codes::ABS_X => (),
            input_event_codes::ABS_Y => (),
            input_event_codes::ABS_Z => (),
            input_event_codes::ABS_RX => (),
            input_event_codes::ABS_RY => (),
            input_event_codes::ABS_RZ => (),
            input_event_codes::ABS_THROTTLE => (),
            input_event_codes::ABS_RUDDER => (),
            input_event_codes::ABS_WHEEL => (),
            input_event_codes::ABS_GAS => (),
            input_event_codes::ABS_BRAKE => (),
            input_event_codes::ABS_HAT0X => (),
            input_event_codes::ABS_HAT0Y => (),
            input_event_codes::ABS_HAT1X => (),
            input_event_codes::ABS_HAT1Y => (),
            input_event_codes::ABS_HAT2X => (),
            input_event_codes::ABS_HAT2Y => (),
            input_event_codes::ABS_HAT3X => (),
            input_event_codes::ABS_HAT3Y => (),
            input_event_codes::ABS_PRESSURE => (),
            input_event_codes::ABS_DISTANCE => (),
            input_event_codes::ABS_TILT_X => (),
            input_event_codes::ABS_TILT_Y => (),
            input_event_codes::ABS_TOOL_WIDTH => (),
            input_event_codes::ABS_VOLUME => (),
            input_event_codes::ABS_MISC => (),
            _ => (),
        },
        input_event_codes::EV_MSC => match code {
            input_event_codes::MSC_TIMESTAMP => (),
            _ => (),
        },
        input_event_codes::EV_SYN => (),
        input_event_codes::EV_SW  => (),
        input_event_codes::EV_LED => (),
        input_event_codes::EV_SND => (),
        input_event_codes::EV_REP => (),
        input_event_codes::EV_FF  => (),
        input_event_codes::EV_PWR => (),
        input_event_codes::EV_FF_STATUS => (),
        _ => (),
    }
}

unsafe fn poll_evdev(evdev: *mut evdev::libevdev) {
    let mut ev: linux_input::input_event = mem::zeroed();
    let status = evdev::libevdev_next_event(evdev, libevdev_read_flag::LIBEVDEV_READ_FLAG_NORMAL as _, &mut ev);
    loop {
        match status {
            s if s == -c::EAGAIN => {
            
            },
            s if s == libevdev_read_status::LIBEVDEV_READ_STATUS_SUCCESS as _ => {
                handle_linux_input_event(&ev);
            },
            s if s == libevdev_read_status::LIBEVDEV_READ_STATUS_SYNC as _ => {
                loop {
                    let status = evdev::libevdev_next_event(evdev, libevdev_read_flag::LIBEVDEV_READ_FLAG_SYNC as _, &mut ev);
                    handle_linux_input_event(&ev);
                    if status == -c::EAGAIN {
                        continue;
                    }
                }
            },
            _ => (),
        };
    }
}

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




unsafe fn rumble(fd: c_int) {
    let mut ff = linux_input::ff_effect {
        type_: ff::FF_RUMBLE,
        id: -1,
        direction: 0,
        trigger: linux_input::ff_trigger {
            button: 0,
            interval: 0,
        },
        replay: linux_input::ff_replay {
            length: 1000, // milliseconds
            delay: 0,
        },
        u: {
            let mut u = linux_input::ff_effect_union::default();
            *u.rumble() = linux_input::ff_rumble_effect {
                strong_magnitude: 0xffff_u16,
                weak_magnitude: 0xffff_u16,
            };
            u
        },
    };

    // Upload the effect. This also changes its id from -1 to some valid value given by the kernel.
    let status = ev_ioctl::register_ff_effect(fd, &mut ff);
    if status.is_err() {
        unimplemented!{};
    }
    if ff.id == -1 {
        // Was not set by the kernel, so we have no way to reference it later!
        unimplemented!{};
    }

    let play = linux_input::input_event {
        type_: input_event_codes::EV_FF,
        code: ff.id as _,
        value: 1,
        time: mem::zeroed(),
    };
    loop {
        let nwritten = c::write(fd, &play as *const _ as _, mem::size_of_val(&play));
        if nwritten == -1 {
            if errno::errno() == c::EAGAIN {
                continue;
            }
            unimplemented!{}
        }
        break;
    }

    let status = ev_ioctl::unregister_ff_effect(fd, ff.id as _);
    if status.is_err() {
        unimplemented!{};
    }
}

