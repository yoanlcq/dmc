use std::os::raw::*;
use std::mem;
use std::ptr;
use std::rc::Rc;
use gl::{GLPixelFormatSettings, GLPixelFormatChooser, GLContextSettings, GLSwapInterval, GLProfile, GLContextResetNotificationStrategy};
use error::{Result, failed};
use super::{OsWindow, OsSharedWindow, winapi_utils::*, wgl::consts::*};

#[derive(Debug)]
pub struct OsGLContext {
    pub window: Rc<OsSharedWindow>,
    pub hglrc: HGLRC,
}

#[derive(Debug)]
pub struct OsGLPixelFormat(c_int);

pub type OsGLProc = extern "system" fn();

impl OsGLContext {
    pub unsafe fn get_proc_address(&self, name: *const c_char) -> *const c_void {
        match wglGetProcAddress(name) as usize {
            0 => GetProcAddress(self.window.context.wgl().unwrap().opengl32_hmodule, name) as *const _, // wglGetProcAddress only works on extension functions
            f => f as *const _,
        }
    }
}

impl OsWindow {
    pub fn create_gl_context(&self, settings: &GLContextSettings) -> Result<OsGLContext> {
        let wgl = self.context.wgl()?;

        let &GLContextSettings {
            version,
            profile,
            debug,
            forward_compatible,
            robust_access,
        } = settings;

        let mut context_attribs = [
            WGL_CONTEXT_MAJOR_VERSION_ARB, version.major as _,
            WGL_CONTEXT_MINOR_VERSION_ARB, version.minor as _,
            WGL_CONTEXT_FLAGS_ARB, 
            (WGL_CONTEXT_DEBUG_BIT_ARB * debug as c_int)
            | (WGL_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB * forward_compatible as c_int)
            | (WGL_CONTEXT_ROBUST_ACCESS_BIT_ARB * robust_access.is_some() as c_int * wgl.WGL_ARB_create_context_robustness as c_int),
            0, 0, // WGL_CONTEXT_PROFILE_MASK_ARB, value,
            0, 0, // WGL_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB, value,
            0, // End
        ];

        let mut i = context_attribs.len() - 5;
        assert_eq!(0, context_attribs[i]);

        if wgl.WGL_ARB_create_context_profile {
            context_attribs[i] = WGL_CONTEXT_PROFILE_MASK_ARB;
            i += 1;
            context_attribs[i] = match profile {
                GLProfile::Core => WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
                GLProfile::Compatibility => WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
            };
            if version.is_es() && (wgl.WGL_EXT_create_context_es_profile || wgl.WGL_EXT_create_context_es2_profile) {
                context_attribs[i] |= WGL_CONTEXT_ES_PROFILE_BIT_EXT;
            }
            i += 1;
        }
        if let Some(robust_access) = robust_access {
            if wgl.WGL_ARB_create_context_robustness {
                context_attribs[i] = WGL_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB;
                i += 1;
                context_attribs[i] = match robust_access {
                    GLContextResetNotificationStrategy::NoResetNotification => WGL_NO_RESET_NOTIFICATION_ARB,
                    GLContextResetNotificationStrategy::LoseContextOnReset => WGL_LOSE_CONTEXT_ON_RESET_ARB,
                };
                i += 1;
            }
        }

        assert_eq!(&0, context_attribs.last().unwrap());
        let hglrc_share: HGLRC = ptr::null_mut();
        let hglrc = unsafe {
            (wgl.fns.wglCreateContextAttribsARB.unwrap())(self.own_dc()?, hglrc_share, context_attribs.as_ptr())
        };
        if hglrc.is_null() {
            winapi_fail("wglCreateContextAttribsARB returned NULL")
        } else {
            Ok(OsGLContext { window: Rc::clone(&self.0), hglrc })
        }
    }
}

impl OsSharedWindow {
    pub fn choose_gl_pixel_format(&self, chooser: &GLPixelFormatChooser) -> Result<OsGLPixelFormat> {
        let wgl = self.context.wgl()?;

        let &GLPixelFormatSettings {
            msaa, depth_bits, stencil_bits, double_buffer, stereo,
            red_bits, green_bits, blue_bits, alpha_bits,
            accum_red_bits,
            accum_green_bits,
            accum_blue_bits,
            accum_alpha_bits,
            aux_buffers,
            transparent,
        } = chooser.settings();

        let mut attribs_i = [
            WGL_DRAW_TO_WINDOW_ARB, TRUE,
            WGL_SUPPORT_OPENGL_ARB, TRUE,
            WGL_DOUBLE_BUFFER_ARB, double_buffer as _,
            WGL_STEREO_ARB, stereo as _,
            WGL_TRANSPARENT_ARB, transparent as _,
            WGL_PIXEL_TYPE_ARB, WGL_TYPE_RGBA_ARB,
            WGL_COLOR_BITS_ARB, (red_bits + green_bits + blue_bits) as _,
            WGL_RED_BITS_ARB, red_bits as _,
            WGL_GREEN_BITS_ARB, green_bits as _,
            WGL_BLUE_BITS_ARB, blue_bits as _,
            WGL_ALPHA_BITS_ARB, alpha_bits as _,
            WGL_DEPTH_BITS_ARB, depth_bits as _,
            WGL_STENCIL_BITS_ARB, stencil_bits as _,
            WGL_ACCUM_RED_BITS_ARB, accum_red_bits as _,
            WGL_ACCUM_GREEN_BITS_ARB, accum_green_bits as _,
            WGL_ACCUM_BLUE_BITS_ARB, accum_blue_bits as _,
            WGL_ACCUM_ALPHA_BITS_ARB, accum_alpha_bits as _,
            WGL_AUX_BUFFERS_ARB, aux_buffers as _,
            0, 0, // WGL_SAMPLE_BUFFERS_ARB, value,
            0, 0, // WGL_SAMPLES_ARB, value,
            0, // End
        ];

        let mut i = attribs_i.len() - 5;
        assert_eq!(0, attribs_i[i]);
        if wgl.WGL_ARB_multisample && msaa.buffer_count > 0 {
            attribs_i[i] = WGL_SAMPLE_BUFFERS_ARB;
            i += 1;
            attribs_i[i] = msaa.buffer_count as _;
            i += 1;
            attribs_i[i] = WGL_SAMPLES_ARB;
            i += 1;
            attribs_i[i] = msaa.sample_count as _;
            i += 1;
        }

        assert_eq!(&0, attribs_i.last().unwrap());
        let attribs_f = &[
            0., // End
        ];
        assert_eq!(&0., attribs_f.last().unwrap());

        let mut candidate_pixel_formats = [0; 32];
        let mut num_formats = 0;
        let is_ok = unsafe {
            (self.context.wgl()?.fns.wglChoosePixelFormatARB.unwrap())(
                self.own_dc()?,
                attribs_i.as_ptr(),
                attribs_f.as_ptr(),
                candidate_pixel_formats.len() as _,
                candidate_pixel_formats.as_mut_ptr(),
                &mut num_formats
            )
        };
        if is_ok == FALSE {
            return winapi_fail("wglChoosePixelFormatARB");
        }
        let candidate_pixel_formats = &candidate_pixel_formats[..num_formats as _];
        let i_pixel_format = candidate_pixel_formats[0]; // FIXME: Use chooser
        assert_ne!(i_pixel_format, 0);
        Ok(OsGLPixelFormat(i_pixel_format))
    }
    pub fn set_pixel_format(&self, pf: &OsGLPixelFormat) -> Result<()> {
        let i_pixel_format = pf.0;
        assert_ne!(i_pixel_format, 0);
        let pfd_kludge = unsafe {
            let mut pfd = PIXELFORMATDESCRIPTOR {
                nSize: mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
                .. mem::zeroed()
            };
            DescribePixelFormat(self.own_dc()?, i_pixel_format, mem::size_of_val(&pfd) as _, &mut pfd);
            pfd
        };
        let is_ok = unsafe {
            SetPixelFormat(self.own_dc()?, i_pixel_format, &pfd_kludge)
        };
        if is_ok == FALSE {
            return winapi_fail("SetPixelFormat");
        }
        Ok(())
    }
    pub fn make_gl_context_current(&self, c: Option<&OsGLContext>) -> Result<()> {
        let hglrc = match c {
            None => ptr::null_mut(),
            Some(c) => c.hglrc,
        };
        let is_ok = unsafe {
            wglMakeCurrent(self.own_dc()?, hglrc)
        };
        if is_ok == FALSE {
            winapi_fail("wglMakeCurrent")
        } else {
            Ok(())
        }
    }
    pub fn gl_swap_buffers(&self) -> Result<()> {
        let is_ok = unsafe { SwapBuffers(self.own_dc()?) };
        if is_ok == FALSE {
            winapi_fail("SwapBuffers")
        } else {
            Ok(())
        }
    }
    pub fn gl_set_swap_interval(&self, interval: GLSwapInterval) -> Result<()> {
        let wgl = self.context.wgl()?;

        let interval = match interval {
            GLSwapInterval::VSync => 1,
            GLSwapInterval::Immediate => 0,
            GLSwapInterval::LateSwapTearing => {
                if !wgl.WGL_EXT_swap_control_tear {
                    return failed("Missing extension `WGL_EXT_swap_control_tear`");
                }
                -1
            },
            GLSwapInterval::Interval(i) => {
                if i < 0 && !wgl.WGL_EXT_swap_control_tear {
                    return failed("Missing extension `WGL_EXT_swap_control_tear`");
                }
                i
            },
        };

        if wgl.WGL_EXT_swap_control {
            let ssi = wgl.fns.wglSwapIntervalEXT.unwrap();
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
        }
    }
}
