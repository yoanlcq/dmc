use std::os::raw::{c_int, c_uchar};
use error::{Result, failed};
use super::context::X11SharedContext;
use super::x11::xlib as x;
use super::x11::xinput2 as xi2;
use super::missing_bits;

#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct XI {
    pub major_opcode: c_int,
    pub event_base: c_int,
    pub error_base: c_int,
    pub major_version: c_int,
    pub minor_version: c_int,
}

impl X11SharedContext {
    pub fn xi(&self) -> Result<&XI> {
        self.xi.as_ref().map_err(Clone::clone)
    }
}

impl XI {
    pub unsafe fn query(x_display: *mut x::Display) -> Result<Self> {
        let mut xi = Self::default();

        let has_it = x::XQueryExtension(
            x_display, b"XInputExtension\0".as_ptr() as *const _,
            &mut xi.major_opcode, &mut xi.event_base, &mut xi.error_base
        );
        if has_it == x::False {
            return failed("XQueryExtension() returned False");
        }

        // NOTE: XGetExtensionVersion() is deprecated for XI2 apps, so let's just not use it.
        // Xinput 2.3 dates back from 2009
        xi.major_version = 2;
        xi.minor_version = 3;
        // returns BadRequest if not supported, may generate BadValue.
        let status = xi2::XIQueryVersion(x_display, &mut xi.major_version, &mut xi.minor_version);
        match status as _ {
            x::Success => (),
            x::BadRequest => return failed("X server doesn't have the XInput 2.3 extension"),
            // They do this in the man page's example
            _ => return failed("XIQueryVersion() returned garbage"),
        }

        let root = x::XDefaultRootWindow(x_display);
        xi_select_events(x_display, root, &[xi2::XIAllDevices], &[
            &[
                xi2::XI_RawKeyPress,
                xi2::XI_RawKeyRelease,
                xi2::XI_RawButtonPress,
                xi2::XI_RawButtonRelease,
                xi2::XI_RawMotion,
                xi2::XI_RawTouchBegin,
                xi2::XI_RawTouchUpdate,
                xi2::XI_RawTouchEnd,
            ]
        ]);

        Ok(xi)
    }
}

pub unsafe fn xi_select_events(x_display: *mut x::Display, x_window: x::Window, devices: &[c_int], events: &[&[c_int]]) {
    assert_eq!(devices.len(), events.len());
    let mut masks_mem = Vec::with_capacity(devices.len());
    let mut masks = Vec::with_capacity(devices.len());
    for i in 0..devices.len() {
        let mask_len = missing_bits::xi::XIMaskLen(xi2::XI_LASTEVENT);
        let mut mask_mem = Vec::<c_uchar>::with_capacity(mask_len as _);
        for ev in events[i] {
            xi2::XISetMask(&mut mask_mem, *ev);
        }
        let mask = mask_mem.as_mut_ptr();
        masks_mem.push(mask_mem);
        masks.push(xi2::XIEventMask {
            deviceid: devices[i],
            mask_len, mask,
        });
    }
    xi2::XISelectEvents(x_display, x_window, masks.as_mut_ptr(), masks.len() as _);
    let _ = masks_mem; // Keep it alive since Non-Lexical Lifetimes???
}
