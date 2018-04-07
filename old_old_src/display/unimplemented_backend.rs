use std::fmt::{self, Formatter};

use super::Extent2;

use super::*;
use super::window::{
    Capabilities,
    WindowOpResult,
};

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Display {
}

#[derive(Debug, PartialEq)]
pub struct Window<'dpy> {
    pub dpy: &'dpy Display,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub(super) struct GLContext<'dpy> {
    pub dpy: &'dpy Display,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct GLPixelFormat<'dpy> {
    pub dpy: &'dpy Display,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Cursor<'dpy> {
    pub dpy: &'dpy Display,
}


#[derive(Debug, Clone)]
pub enum Error {
    FunctionName(&'static str),
}

impl Drop for Display {
    fn drop(&mut self) {
        unimplemented!{}
    }
}

impl<'dpy> Drop for Window<'dpy> {
    fn drop(&mut self) {
        unimplemented!{}
    }
}

impl<'dpy> Drop for GLPixelFormat<'dpy> {
    fn drop(&mut self) {
        unimplemented!{}
    }
}

impl<'dpy> Drop for GLContext<'dpy> {
    fn drop(&mut self) {
        unimplemented!{}
    }
}





impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            Error::FunctionName(name) => {
                write!(fmt, "{}() failed", name)
            },
        }
    }
}

impl Display {

    pub(super) fn open() -> Result<Self, super::Error> {
        // Just to disable macro_use warning.
        warn!("Sorry, this is not implemented yet!");
        unimplemented!{}
    }

    pub fn allow_session_termination(&mut self) -> Result<(), Error> {
        Err(super::Error::Unimplemented)
    }
    pub fn disallow_session_termination(&mut self, reason: Option<&str>) -> Result<(), Error> {
        Err(super::Error::Unimplemented)
    }

    pub(super) fn choose_gl_pixel_format<'dpy>(&'dpy self, _settings: &GLPixelFormatSettings)
        -> Result<GLPixelFormat<'dpy>, super::Error>
    {
        unimplemented!{}
    }

    pub(super) fn create_window<'dpy>(&'dpy self, _settings: &Settings) 
        -> Result<Window<'dpy>, super::Error>
    {
        unimplemented!{}
    }

    pub(super) fn create_gl_context<'dpy>(&'dpy self, _pf: &GLPixelFormat, _cs: &GLContextSettings) 
        -> Result<GLContext<'dpy>, super::Error>
    {
        unimplemented!{}
    }


    pub(super) fn create_software_gl_context<'dpy>(&'dpy self, _pf: &GLPixelFormat, _cs: &GLContextSettings) 
        -> Result<GLContext<'dpy>,super::Error>
    {
        unimplemented!()
    }
}


impl<'dpy> GLContext<'dpy> {
    pub(super) fn make_current(&self, _win: &Window) {
        unimplemented!()
    }

    pub(super) unsafe fn get_proc_address_raw(&self, _name: *const c_char) -> Option<unsafe extern "C" fn()> {
        unimplemented!()
    }
    pub(super) fn get_proc_address(&self, _name: &str) -> Option<unsafe extern "C" fn()> {
        unimplemented!()
    }
}


impl<'dpy> Window<'dpy> {

    pub(super) fn gl_swap_buffers(&self) {
        unimplemented!()
    }

    pub(super) fn gl_set_swap_interval(&self, _interval: GLSwapInterval) -> Result<(),super::Error> { 
        unimplemented!()
    }

    pub(super) fn get_capabilities(&self) -> Capabilities {
        unimplemented!()
    }

    pub(super) fn show(&self) -> WindowOpResult<()> {
        unimplemented!()
    }
    pub(super) fn create_child(&self, _settings: &Settings) -> Result<Self, super::Error> {
        unimplemented!()
    }
    pub(super) fn hide(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn maximize(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn minimize(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn raise(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn restore(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn set_style(&self, _style: &Style) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn enter_fullscreen(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn leave_fullscreen(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn clear_icon(&self) -> WindowOpResult<()> {
        unimplemented!()
    }
    pub(super) fn set_icon(&self, _icon: Icon) -> WindowOpResult<()> {
        unimplemented!()
    }
    pub(super) fn set_minimum_size(&self, _size: Extent2<u32>) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn set_maximum_size(&self, _size: Extent2<u32>) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn set_opacity(&self, _opacity: f32) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn move_absolute(&self, _pos: Extent2<u32>) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn move_relative_to_self(&self, _pos: Extent2<u32>) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn move_relative_to_parent(&self, _pos: Extent2<u32>) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn recenter(&self) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn resize(&self, _size: Extent2<u32>) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn set_title(&self, _title: &str) -> WindowOpResult<()> {
        unimplemented!()
    }
    pub(super) fn query_screenspace_size(&self) -> Extent2<u32> {
        unimplemented!()
    }
    pub(super) fn query_canvas_size(&self) -> Extent2<u32> {
        unimplemented!()
    }
}

