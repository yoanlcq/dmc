use std::rc::Rc;
use std::ffi::CStr;
use std::path::Path;
use std::os::raw::c_char;
use gl::*;
use window::*;
use cursor::*;
use super::{Extent2, Vec2};

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
#[derive(Debug)]
pub struct OsWindow {}
#[derive(Debug)]
pub struct OsGLContext {}
#[derive(Debug, PartialEq)]
pub struct OsGLPixelFormat {}
#[derive(Debug)]
pub struct OsCursor {}

use context::Error;

impl OsContext {
    pub fn open() -> Result<Self, Error> { unimplemented!{} }
    pub fn open_x11_display_name(name: Option<&CStr>) -> Result<Self, Error> { unimplemented!{} }
    pub fn create_window(&mut self, settings: &WindowSettings) -> Result<OsWindow, Error> { unimplemented!{} }
    pub fn create_window_and_show(&mut self, settings: &WindowSettings) -> Result<OsWindow, Error> { unimplemented!{} }
    pub fn choose_gl_pixel_format(&self, settings: &GLPixelFormatSettings) -> Result<OsGLPixelFormat, Error> { unimplemented!{} }
    pub fn create_gl_context(&self, pf: &OsGLPixelFormat, cs: &GLContextSettings) -> Result<OsGLContext, Error> { unimplemented!{} }
    pub fn create_software_gl_context(&self, pf: &OsGLPixelFormat, cs: &GLContextSettings) -> Result<OsGLContext, Error> { unimplemented!{} }
    pub fn create_gl_context_from_lib<P: AsRef<Path>>(&self, _pf: &OsGLPixelFormat, _cs: &GLContextSettings, _path: P) -> Result<OsGLContext, Error> { unimplemented!{} }
    pub fn allow_session_termination(&mut self) -> Result<(), Error> { unimplemented!{} }
    pub fn disallow_session_termination(&mut self, reason: Option<String>) -> Result<(), Error> { unimplemented!{} }
    pub fn query_best_cursor_size(&self, _size_hint: Extent2<u32>) -> Extent2<u32> { unimplemented!{} }
    pub fn create_cursor(&self, _img: CursorFrame) -> Result<OsCursor, Error> { unimplemented!{} }
    pub fn create_animated_cursor(&self, _anim: &[CursorFrame]) -> Result<OsCursor, Error> { unimplemented!{} }
    pub fn system_cursor(&self, _s: SystemCursor) -> Result<OsCursor, Error> { unimplemented!{} }
}

impl OsWindow {
    pub fn show(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn hide(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn set_title(&self, title: &str) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn set_icon(&self, icon: Icon) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn clear_icon(&self) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn set_style(&self, style: &WindowStyle) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn recenter(&self) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn set_opacity(&self, opacity: f32) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn query_screenspace_size(&self) -> Extent2<u32> {
        unimplemented!{}
    }
    pub fn query_canvas_size(&self) -> Extent2<u32> {
        unimplemented!{}
    }
    pub fn maximize(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn minimize(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn restore(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn raise(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn enter_fullscreen(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn leave_fullscreen(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn set_minimum_size(&self, size: Extent2<u32>) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn set_maximum_size(&self, size: Extent2<u32>) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn move_absolute(&self, pos: Extent2<u32>) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn move_relative_to_self(&self, pos: Extent2<u32>) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn move_relative_to_parent(&self, pos: Extent2<u32>) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn resize(&self, size: Extent2<u32>) -> Result<(), Error> {
        unimplemented!{}
    }

    // XQueryBestCursor
    // XCreatePixmapCursor
    // XDefineCursor, XUndefineCursor
    // XRecolorCursor
    // XFreeCursor
    pub fn is_cursor_shown(&self) -> Result<bool, Error> { unimplemented!{} }
    pub fn show_cursor(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn hide_cursor(&self) -> Result<(), Error> { unimplemented!{} }
    pub fn set_cursor(&self, _cursor: &Cursor) -> Result<(), Error> { unimplemented!{} }
    pub fn cursor(&self) -> Result<&Cursor, Error> { unimplemented!{} }
    pub fn set_cursor_position(&self, _pos: Vec2<u32>) -> Result<(), Error> { unimplemented!{} }
    pub fn cursor_position(&self) -> Result<Vec2<u32>, Error> { unimplemented!{} }

    pub fn make_gl_context_current(&self, gl_context: Option<&OsGLContext>) { unimplemented!{} }
    pub fn gl_swap_buffers(&self) {
        unimplemented!{}
    }
    pub fn set_gl_swap_interval(&mut self, interval: GLSwapInterval) -> Result<(), Error> {
        unimplemented!{}
    }
}

impl OsGLContext {
    pub fn get_proc_address(&self, name: &str) -> Option<unsafe extern "C" fn()> { unimplemented!{} }
    pub unsafe fn get_proc_address_raw(&self, name: *const c_char) -> Option<unsafe extern "C" fn()> { unimplemented!{} }
}

impl OsHid {
    pub fn is_connected(&self) -> bool {
        unimplemented!{}
    }
}

