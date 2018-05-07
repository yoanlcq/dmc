//! Module for common human interface devices (HID).
//!
//! This module is named "HID" because it is a short and nice term for
//! human input/output devices, which excludes e.g hard drives, usb disks, GPUs, etc.  
//!
//! "Device" is a very generic term that could even qualify consoles, mobile phones and
//! computers, but "HID" is more specific and includes mice, keyboards, gamepads and so on.
//!
//! For a rationale, see [Wikipedia's definition](https://en.wikipedia.org/wiki/Human_interface_device).
//!
//! A master HID is the "physical" counterpart to what is called an HID in this module.  
//! To illustrate, a graphics tablet may be reported as two devices: a `Touch` and a `Tablet`,
//! but both really are features of a single physical device; this physical device is the master
//! HID.
//!
//! F.A.Q
//!
//! # What's in a `HidID` ?
//!
//! Essentially, two parts :
//! - An actual, backend-specific ID; this ID is only valid as long as the device is "alive".
//! - A unique token, generated by this crate as the device is plugged for the first time.
//!
//! In order to understand the point of the token, picture the following:
//!
//! 1. User plugs some gamepad; the backend reports an ID equal to, say, 13.
//! 2. Your application keeps a copy of the ID, assuming it always refers to that gamepad.
//! 3. User unplugs the gamepad, and then plugs some mouse.
//!    The backend decides that the ID `13` is free for reuse since the gamepad was unplugged,
//!    and decides to use it for the mouse now.
//! 4. What your application thinks is a gamepad is now a mouse. As your application makes further
//!    queries, this crate thinks "Errm dude stop this ain't a gamepad no more", and your
//!    application thinks "Why does this fail? This ID definitely refers to a
//!    gamepad, I've seen it work the first time; this crate sucks?? :(".
//!
//! The token gives this crate the ability to detect when your aplication is
//! using a stale ID. What your app should do in this case is just drop the ID because
//! it obviously breaks assumptions.  
//! Also, either way, your app should handle `HidDisconnected` events and drop
//! the associated IDs.
//!
//!
//! # Why aren't there strongly-typed ID types for each device kind?
//!
//! This was my first approach and I ended up moving away from it, for these reasons:
//!
//! - This was a actually more of a pain to deal with than it's worth.
//!   It makes for too many type aliases to remember and cope with all the time.
//! - It's a chore to deal with as more device kinds are added. In other words, it does not scale well.
//! - Actually, we can't just classify a device as being of one kind AND NOT any other.
//!   In fact, the proper way to classify devices is not the first one that comes to mind!
//!   See next question.
//!
//!
//! # Why aren't there separate, well-defined device kinds ? There's no way my mouse is also a keyboard!
//!
//! Famous last words! :) Here are the reasons there is no `enum DeviceKind { ... }` :
//!
//! Often, backends do not provide such exact information; What we can ask them is not "Are you
//! a ...?" but "Do you look like a ...?", "Do you behave like a ...?".
//! Essentially, they allow device kinds to be expressed as
//! a set of flags rather than a single discriminant value.  
//! Converting a set of flags into a single value is loss of information, plain and
//! simple; which we definitely do not want.
//!
//! There's also the case of pen tablets; they are treated as mice by the backend (in fact, they
//! are litterally replacements for mice; that's the whole point), AND some of them are also
//! touch surfaces. However there are definitely mice that are not tablets, and touch surfaces that
//! are not tablets either.
//!
//! It gets worse on Linux:
//! `udev` doesn't prevent a controller device from reporting itself as both a joystick and a gamepad, or both a steering wheel and a
//! gamepad, etc. You can't just say "this is only a gamepad" or "this is only a steering wheel",
//! even though **in practice**, **in the average case**, you might.
//!
//! Also on Linux, there is `uinput` which allows users to create their own virtual devices however
//! they like. The consequences? **A device might be a mouse, keyboard, tablet, and gamepad all
//! at the same time**, and this would be **wanted** by the end user.
//!
//! You are always free to make your own `enum` for distinct device types if you find it convenient
//! in your application, but at least the loss of information is under your control, not enforced
//! by this crate.
//!
//! # What's a master HID ?
//!
//! This concept exists on X11 with the XInput extension.
//!
//! Basically you may have multiple devices that behave like a mouse (say, the touchpad on a laptop
//! + an USB mouse + a pen tablet), all connected and working at the same time. However, they all
//! control a common, single _mouse pointer_, the one you see on the screen.
//!
//! XInput defines that mouse pointer as a _master_ device; your actual devices are defined as
//! _slave_ devices that have (or not) a say in how the _master_ device behaves.
//!
//! This model allows having multiple mouse pointers on a user session instead of only one (this is referred to as MPX
//! (Multi-Pointer eXtension)), and the
//! physical devices may be attached to one or another of them; You may therefore have one cursor controlled
//! by a mouse, and one cursor controlled by a pen tablet.
//!
//! The same applies for keyboards.
//!
//!
//! # What's a parent HID ?
//!
//! This concept exists at least on Linux. This corresponds to whatever the backend reports as
//! being a "parent" device.
//! For instance, a single pen tablet may be reported as multiple "child" devices: A touch surface,
//! a mouse, and button pad. The parent of any of these should be the pen tablet itself, 
//! hopefully reported as a single, normal device stripped of its capabilities (because they're in the
//! children) but this is unclear; it depends too much on obscure platform-specific and
//! driver-specific behaviour.
//!
//! Anyway this crate exposes this features because it might be useful.
//!
//!
//! # Why expose so much information about devices?
//!
//! My mindset is "if it exists and is exposed, it's going to be useful for someone".
//!
//! It's true that this _does_ eat up some memory, but in all honesty, this is nothing compared
//! to what a decent application allocates by itself. Even if you don't use this information,
//! it already lives in the operating system kernel's memory _anyway_. We're only doing what a
//! normal library does by prefetching everything we need into a convenient representation.
//!
//! Why would anyone care about the driver's name? Well, buggy drivers do (and will) exist, and
//! users only care as long as the application they want to use "works". Working around
//! buggy/quirky drivers is definitely a thing, and it's hard to do without any hint.
//!
//! Knowing about the `Bus` may be useful if your application wants to display a nice icon
//! next to some dropdown list of connected devices.
//!
//! The same applies to anything backends are able to report.
//!
//!
//! # Why prefetch so much information eagerly?
//!
//! Mostly convenience for the implementation. Other reasons include :
//! - Predictable memory allocation patterns. If the information was returned "lazily",
//!   there would be allocations multiple times instead of once.
//! - Separation of concerns.  
//!   There are two kinds of queries: those that query real-time
//!   state (Is the device plugged? What's the state of this button? etc) and those
//!   that query some kind of static database (typically, the device's information, including its
//!   capabilities).
//!   Said database is _supposed_ to outlive the device (which is the case for some backends).
//!   By prefetching everything ASAP and keeping it warm for later, this crate gives you
//!   the opportunity to keep using a device's information even after it is unplugged.
//!
//!
//! # Why are axis values `f64`?
//! 
//! Because this is the widest number type, so it is an excellent common denominator across
//! implementations. Because of its width, conversion from 16-bit and 32-bit integers (which is what backends often report)
//! is lossless.
//!
//! Beware though, this never means that values are "normalized" (whatever that means).
//! A value is only meaningful in the context of an `AxisInfo` and the process of "normalizing" it
//! is up to you.
//!
//! Using a newtype would be pointless here, because it would have to provide `to_f64()` and
//! `to_i32()` methods, but `f64`-to-`i32` conversion is lossy. Let's just use `f64` and call it a
//! day.

use uuid::Uuid as Guid;
use std::path::PathBuf;
use std::ops::{Range, Not};
use context::Context;
use event::Timestamp;
use os::OsHidID;

pub mod mouse;
pub use self::mouse::*;
pub mod keyboard;
pub use self::keyboard::*;
pub mod touch;
pub use self::touch::*;
pub mod tablet;
pub use self::tablet::*;
pub mod controller;
pub use self::controller::*;

/// Error returned by operations from this module and submodules.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Error {
    /// The device was disconnected at the specific timestamp, if known.
    DeviceDisconnected(Option<Timestamp>),
    /// The device (or backend for the device) does not support this operation.
    NotSupportedByDevice { reason: Option<super::error::CowStr> },
    /// Another error occured (in the meantime, it is unknown whether or not the device is still connected).
    Other(super::error::Error),
}

/// Convenience alias to `Result<T, Error>`.
pub type Result<T> = ::std::result::Result<T, Error>;

#[allow(dead_code)]
pub(crate) fn disconnected<T>(timestamp: Timestamp) -> Result<T> {
    Err(Error::DeviceDisconnected(Some(timestamp)))
}
#[allow(dead_code)]
pub(crate) fn disconnected_no_timestamp<T>() -> Result<T> {
    Err(Error::DeviceDisconnected(None))
}

#[allow(dead_code)]
pub(crate) fn not_supported_by_device<T, S: Into<super::error::CowStr>>(s: S) -> Result<T> {
    Err(Error::NotSupportedByDevice { reason: Some(s.into()) })
}
#[allow(dead_code)]
pub(crate) fn not_supported_by_device_unexplained<T>() -> Result<T> {
    Err(Error::NotSupportedByDevice { reason: None })
}



/// A button or key state, i.e "up" or "down".
///
/// This type exists only because a `bool` is not explicit enough;
/// `some_key.is_down()` and `some_key.is_up()` is more explicit
/// and less error-prone.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum ButtonState {
    /// The button or key is being held down.
    Down,
    /// The button or key is up.
    Up,
}
/// A key is mostly the same thing as a button.
pub type KeyState = ButtonState;

impl ButtonState {
    /// Is the button or key being held down?
    pub fn is_down(&self) -> bool {
        self == &ButtonState::Down
    }
    /// Is the button or key up?
    pub fn is_up(&self) -> bool {
        self == &ButtonState::Up
    }
    /// Flips this button state.
    pub fn flip(&mut self) { 
        *self = !*self;
    }
    /// This button state, flipped.
    pub fn flipped(&self) -> Self { 
        !*self
    }
}

/// The `!` operator flips a `ButtonState`.
impl Not for ButtonState {
    type Output = Self;
    fn not(self) -> Self {
        match self {
            ButtonState::Down => ButtonState::Up,
            ButtonState::Up => ButtonState::Down,
        }
    }
}

/// Information about an absolute unidimensional axis.
///
/// The content of this `struct` is mostly taken from Linux's `input_absinfo` `struct`.
///
/// Multidirectional axii will want to expose one `AxisInfo` per dimension.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisInfo {
    /// The minimum and maximum values this axis can take.
    pub range: Range<f64>,
    /// A range within which input should probably be ignored.
    ///
    /// Not all backends expose this, but if they do, you should not treat the value as truthful,
    /// because even drivers may get it wrong.
    ///
    /// Therefore, you should take it only as a hint, but still allow it to be configurable in your app.
    pub dead_zone: Option<Range<f64>>,
    /// Resolution, in units per millimeter, or units per radian.
    ///
    /// This is mostly Linux-specific and probably not useful.
    pub resolution: f64,
    /// Linux-specific, fuzz value that is used to filter noise from the event stream.
    /// 
    /// The input system in Linux will drop events generated by the device driver
    /// if the difference from the last value is lower than the fuzz value.
    ///
    /// (See this StackOverflow answer)[https://stackoverflow.com/a/17041513/7972165].
    pub fuzz: f64,
}

/// Information about a HID (often fetched once when the device is detected).
///
/// A lot of these fields are optional because not all backend (and not all drivers)
/// provide the same amount of information. In addition, some devices are somewhat "virtual"
/// (for instance, the "core" pointer and keyboard in X11) and provide very little
/// information about them (only a name, for instance).
///
/// So, this is a bit cumbersome to work with, but this is indeed the common denominator
/// across platforms, backends, drivers and devices.
///
/// There's normally no point in having so much information about a device, but this might be
/// useful for several purposes:
///
/// - Displaying the friendly name for this device to the user;
/// - Knowing if this is a master device; If it is, the `master` member is `None`.
/// - Identifying a very specific device model, via the USB product info;
///   This is how you could detect that a controller is in fact a "XBox One S" gamepad, for instance.
/// - Identifying the driver, which could be useful for bug reports and patching your application
///   accordingly if the driver is known to be buggy;
#[derive(Debug, Clone, PartialEq)]
pub struct HidInfo {
    /// The master HID, if any.
    pub master: Option<HidID>,
    /// The parent HID, if any.
    pub parent: Option<HidID>,
    /// On Unices, a device is also a file, e.g `/dev/input/event13`.
    pub device_node: Option<PathBuf>,
    /// General-purpose, user-friendly name for this device.
    pub name: Option<String>,
    /// Generic serial string for this device. This is whatever the backend or driver advertises as
    /// a "serial".
    pub serial: Option<String>,
    /// USB product info, if any.
    pub usb_product_info: Option<UsbProductInfo>,
    /// GUID for this device, if any. This is normally only relevant for Windows.
    pub guid: Option<Guid>,
    /// The time at which this device was first plugged.
    pub plug_timestamp: Option<Timestamp>,
    /// The bus by which this device is connected.
    pub bus: Option<Bus>,
    /// The name of the driver, as advertised by the backend.
    pub driver_name: Option<String>,
    /// The driver version in its platform-specific representation.
    /// This is a string because there's no uniform representation for driver versions across
    /// platforms.
    pub driver_version: Option<String>,
    /// Does this device denote an actual, physical one?
    ///
    /// This is a pretty tough question to answer and the meaning, albeit vague, is not
    /// related to the notion of master devices.
    ///
    /// For instance, Linux has `uinput` which allows users to create virtual devices.
    /// These are NOT master devices, but they still don't have a physical form.
    ///
    /// This member is mostly a hint for you, the application writer, for whatever purpose
    /// feels relevant to you.
    pub is_physical: Option<bool>,
    /// If this device is a controller, then controller-specific info is stored here.
    pub controller: Option<ControllerInfo>,
    /// If this device is a mouse, then mouse-specific info is stored here.
    pub mouse: Option<MouseInfo>,
    /// If this device is a keyboard, then keyboard-specific info is stored here.
    pub keyboard: Option<KeyboardInfo>,
    /// If this device is a touch screen/pad, then specific info is stored here.
    pub touch: Option<TouchInfo>,
    /// If this device is a tablet, then tablet-specific info is stored here.
    pub tablet: Option<TabletInfo>
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UsbProductInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub vendor_name: String,
    pub product_name: String,
}

/// Mostly taken from the `BUS_*` constants in Linux's `input.h`.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Bus {
    Pci,
    Usb,
    Bluetooth,
    Virtual,
}

/// An ID for a HID.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct HidID(pub(crate) OsHidID);

impl Context {
    /// Get the `HidInfo` for the given device ID.
    fn hid_info(&self, id: HidID) -> Result<HidInfo> {
        self.0.hid_info(id)
    }
    /// Checks if the given device is still connected.
    fn ping_hid(&self, id: HidID) -> Result<()> {
        self.0.ping_hid(id)
    }
}
