use std::os::raw::c_int;
use std::mem;
use error::{Result, failed};
use super::context::X11SharedContext;
use super::x11::xrender;
use super::x11::xlib as x;
use super::missing_bits;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct XRender {
    pub major_version: c_int,
    pub minor_version: c_int,
    pub error_base: c_int,
    pub event_base: c_int,
    // Spec doesn't say if these should be freed
    pub argb32_pict_format: *mut xrender::XRenderPictFormat,
    pub rgb24_pict_format: *mut xrender::XRenderPictFormat,
}

impl X11SharedContext {
    pub fn xrender(&self) -> Result<&XRender> {
        self.xrender.as_ref().map_err(Clone::clone)
    }
}

impl XRender {
    pub unsafe fn query(x_display: *mut x::Display) -> Result<Self> {
        let mut xrender = Self { .. mem::zeroed() };

        let has_it = xrender::XRenderQueryExtension(x_display, &mut xrender.error_base, &mut xrender.event_base);
        if has_it == x::False {
            return failed("XRenderQueryExtension() returned False");
        }

        let success = xrender::XRenderQueryVersion(x_display, &mut xrender.major_version, &mut xrender.minor_version);
        if success == 0 {
            return failed("XRenderQueryVersion() returned 0");
        }
        xrender.argb32_pict_format = xrender::XRenderFindStandardFormat(
            x_display, missing_bits::xrender::PictStandard::ARGB32 as _
        );
        xrender.rgb24_pict_format = xrender::XRenderFindStandardFormat(
            x_display, missing_bits::xrender::PictStandard::RGB24 as _
        );
        Ok(xrender)
    }
}
