use std::os::raw::c_char;
use gl::{GLPixelFormatSettings, GLContextSettings, GLSwapInterval};
use error::Result;
use super::{OsContext, OsWindow};

#[derive(Debug)]
pub struct OsGLContext;
pub type OsGLPixelFormat = ();
pub type OsGLProc = ();

impl OsContext {
    pub fn choose_gl_pixel_format(&self, settings: &GLPixelFormatSettings) -> Result<OsGLPixelFormat> {
        unimplemented!()
    }
    pub fn create_gl_context(&self, settings: &GLContextSettings, pf: &OsGLPixelFormat) -> Result<OsGLContext> {
        unimplemented!()
    }
    pub fn make_gl_context_current(&self, w: &OsWindow, c: Option<&OsGLContext>) -> Result<()> {
        unimplemented!()
    }
}

impl OsGLContext {
    pub unsafe fn get_proc_address(&self, name: *const c_char) -> Option<OsGLProc> {
        unimplemented!()
    }
}

impl OsWindow {
    pub fn gl_swap_buffers(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn gl_set_swap_interval(&self, interval: GLSwapInterval) -> Result<()> {
        unimplemented!()
    }
}

