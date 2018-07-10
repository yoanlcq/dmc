use std::os::raw::c_int;
use std::mem;
use error::{Result, failed};
use super::context::X11SharedContext;
use super::xlib_error;
use super::missing_bits;
use super::x11::xlib as x;
use super::x11::xinput2 as xi2;

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
    pub fn xi_select_all_non_raw_events_all_devices(&self, x_window: x::Window) -> Result<()> {
        if let Err(e) = self.xi() {
            return Err(e);
        }
        unsafe {
            xi_select_events(*self.lock_x_display(), x_window, &[(
                xi2::XIAllDevices,
                &[
                    xi2::XI_ButtonPress,
                    xi2::XI_ButtonRelease,
                    // xi2::XI_KeyPress, // Do not subscribe to XI_KeyPress; It replaces core
                    // KeyPress events, which we have to rely on to get proper results with XUtf8LookupString (especielly with compose, e.g "^e" => "ê")
                    // xi2::XI_KeyRelease,
                    xi2::XI_Motion,
                    xi2::XI_DeviceChanged,
                    xi2::XI_Enter,
                    xi2::XI_Leave,
                    xi2::XI_FocusIn,
                    xi2::XI_FocusOut,
                    xi2::XI_TouchBegin,
                    xi2::XI_TouchUpdate,
                    xi2::XI_TouchEnd,
                    xi2::XI_HierarchyChanged,
                    xi2::XI_PropertyEvent,
                ]
            )])
        }
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
            x::BadRequest => return failed(format!("X server doesn't have the XInput {}.{} extension", xi.major_version, xi.minor_version)),
            // They do this in the man page's example
            _ => return failed("XIQueryVersion() returned garbage"),
        }

        let root = x::XDefaultRootWindow(x_display);
        let status = xi_select_events(x_display, root, &[(
            xi2::XIAllDevices,
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
        )]);

        if let Err(e) = status {
            error!("Could not select all XI raw events for XIAllDevices: {}", e);
        }

        Ok(xi)
    }
}

pub unsafe fn xi_select_events(x_display: *mut x::Display, x_window: x::Window, devices_events: &[(c_int, &[c_int])]) -> Result<()> {
    assert!(!x_display.is_null());
    let mut masks_mem = vec![mem::zeroed(); devices_events.len()];
    let mut masks = vec![mem::zeroed(); devices_events.len()];
    for (deviceid, events) in devices_events.iter().cloned() {
        let mask_len = missing_bits::xi::XIMaskLen(xi2::XI_LASTEVENT);
        let mut mask_mem = vec![0; mask_len as usize];
        for ev in events {
            // These are macro translations and not actual Xlib calls, so no need to catch errors.
            xi2::XISetMask(&mut mask_mem, *ev);
        }
        let mask = mask_mem.as_mut_ptr();
        masks_mem.push(mask_mem);
        masks.push(xi2::XIEventMask { deviceid, mask_len, mask });
    }
    let status = xlib_error::sync_catch(x_display, || {
        xi2::XISelectEvents(x_display, x_window, masks.as_mut_ptr(), masks.len() as _)
    });
    let _ = masks_mem; // Keep it alive since Non-Lexical Lifetimes???
    match status {
        Err(e) => failed(format!("XISelectEvents generated {}", e)),
        Ok(status) => if status != x::Success as _ {
            failed(format!("XISelectEvents returned {}", status))
        } else {
            Ok(())
        },
    }
}
