use std::os::raw::c_char;
use gl::{GLPixelFormatSettings, GLContextSettings, GLSwapInterval};
use error::{Result, failed};
use super::{OsContext, OsWindow, winapi_utils::*};

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
        let hdc = unimplemented!();
        let is_ok = unsafe { SwapBuffers(hdc) };
        if is_ok == FALSE {
            winapi_fail("SwapBuffers")
        } else {
            Ok(())
        }
    }
    pub fn gl_set_swap_interval(&self, interval: GLSwapInterval) -> Result<()> {
        unimplemented!()
        /*
        let wgl = self.context.wgl()?;

        let interval = match interval {
            GLSwapInterval::VSync => 1,
            GLSwapInterval::Immediate => 0,
            GLSwapInterval::LateSwapTearing => {
                if !wgl.ext.WGL_EXT_swap_control_tear {
                    return failed("Missing extension `WGL_EXT_swap_control_tear`");
                }
                -1
            },
            GLSwapInterval::Interval(i) => {
                if i < 0 && !wgl.ext.WGL_EXT_swap_control_tear {
                    return failed("Missing extension `WGL_EXT_swap_control_tear`");
                }
                i
            },
        };

        if wgl.ext.WGL_EXT_swap_control {
            let ssi = wgl.ext.wglSwapIntervalEXT.unwrap();
            let is_ok = unsafe {
                ssi(interval)
            };
            if is_ok == FALSE {
                winapi_fail("wglSwapIntervalEXT")
            } else {
                Ok(())
            }
        } else {
            failed("There's no extension that could set the swap interval!")
        }*/
    }
}

