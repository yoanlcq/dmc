// Depending on the GLX version:
//
// - All versions :
//   - Always use glXGetProcAddressARB (glXGetProcAddress is not always 
//     exported);
//   - GLX_ARB_multisample
//   - GLX_EXT_swap_control
//   - GLX_EXT_swap_control_tear
//   - GLX_MESA_swap_control
//   - GLX_SGI_swap_control
// - 1.1
//   - glxChooseVisual (log glXGetConfig) + glXCreateContext;
// - 1.3
//   - glXChooseFBConfig (log glXGetFBConfigAttrib) + glxCreateNewContext;
// - 1.4
//   - GLX_SAMPLE_BUFFERS, GLX_SAMPLES (formerly ext GLX_ARB_multisample)
//   - try glXCreateContextAttribsARB, otherwise same as 1.3;
//   - GLX_CONTEXT_ROBUST_ACCESS_BIT_ARB
//   - GLX_EXT_create_context_es_profile
//   - GLX_EXT_create_context_es2_profile

use std::os::raw::{c_void, c_char};
use std::rc::Rc;
use std::ptr;
use std::slice;
use version_cmp;
use super::x11::xlib as x;
use super::x11::glx::*;
use super::{X11Context, X11SharedContext, X11SharedWindow};
use super::xlib_error;
use gl::{GLPixelFormatChooser, GLContextSettings, GLSwapInterval};
use error::{Result, failed};

#[derive(Debug)]
pub struct X11GLContext {
    pub context: Rc<X11SharedContext>,
    pub glx_context: GLXContext,
}

#[derive(Debug)]
pub struct X11GLPixelFormat {
    pub context: Rc<X11SharedContext>,
    pub visual_info: *mut x::XVisualInfo,
    pub fbconfig: Option<GLXFBConfig>, // GLX >= 1.3
}

impl Drop for X11GLPixelFormat {
    fn drop(&mut self) {
        unsafe {
            x::XFree(self.visual_info as *mut _); // NOTE: Fine to do on NULL.
        }
    }
}

impl Drop for X11GLContext {
    fn drop(&mut self) {
        unsafe {
            // Defers destruction until it's not current to any thread.
            glXDestroyContext(*self.context.lock_x_display(), self.glx_context);
        }
    }
}



impl X11GLContext {
    pub unsafe fn get_proc_address(&self, name: *const c_char) -> *const c_void {
        #[cfg(not(target_os = "linux"))]
        unimplemented!("We don't know how the situation is in OSes other than Linux! This could require moving to x11-dl.");

        #[cfg(target_os = "linux")]
        match glXGetProcAddressARB(name as _) {
            None => ptr::null(),
            Some(p) => p as _,
        }
    }
}


impl X11Context {
    pub fn choose_gl_pixel_format(&self, chooser: &GLPixelFormatChooser) -> Result<X11GLPixelFormat> {
        let glx = self.glx()?;
        let x_display = self.lock_x_display();
        let settings = chooser.settings();

        if version_cmp::lt((glx.major_version, glx.minor_version), (1, 3)) {
            // Not actually mutated, but glXChooseVisual wants *mut...
            let mut visual_attribs = glx.gen_visual_attribs(settings);
            let visual_info = unsafe {
                glXChooseVisual(*x_display, self.x_default_screen_num(), visual_attribs.as_mut_ptr())
            };
            if visual_info.is_null() {
                return failed("glXChooseVisual() returned NULL");
            }
            return Ok(X11GLPixelFormat { context: Rc::clone(&self.0), visual_info, fbconfig: None });
        }

        // If we're here, we have GLX >= 1.3.

        let visual_attribs = glx.gen_fbconfig_attribs(settings);
        let fbconfigs = {
            let mut fbcount = 0;
            let fbcs = unsafe { 
                glXChooseFBConfig(*x_display, self.x_default_screen_num(), visual_attribs.as_ptr(), &mut fbcount)
            };
            if fbcs.is_null() {
                return failed("glXChooseFBConfig() returned NULL");
            }
            if fbcount <= 0 {
                return failed("glXChooseFBConfig() returned zero FBConfigs");
            }
            unsafe {
                slice::from_raw_parts(fbcs, fbcount as _)
            }
        };

        // fbcs is an array of candidates, from which we choose the best.
        //
        // glXChooseFBConfig's man page describes the sorting order, which
        // in general favors more lightweight configs: what matters most to us
        // is that the sorting order favors single-buffered configs, and is
        // apparently oblivious to MSAA parameters.
        //
        // So what we've got to do is run through the list of candidates and
        // stop at the first that supports double buffering and exactly our
        // MSAA params. If there's none, we'll just select the first one.

        // NOTE: Because of this, I'm completely ignoring the `chooser`'s object's opinion.
        // I won't bother to do the best-to-worst sorting myself.

        let mut best_fbc = fbconfigs[0];
        let mut best_fbc_i = 0;
        let mut is_fbconfig_chosen = false;

        for (i, fbc) in fbconfigs.iter().enumerate() {
            let visual_info = unsafe {
                glXGetVisualFromFBConfig(*x_display, *fbc)
            };
            if visual_info.is_null() {
                continue;
            }
            let mut sample_buffers          = 0;
            let mut samples                 = 0;
            let mut fbconfig_id             = 0; 
            let mut buffer_size             = 0; 
            let mut level                   = 0; 
            let mut stereo                  = 0; 
            let mut doublebuffer            = 0;
            let mut aux_buffers             = 0; 
            let mut red_size                = 0; 
            let mut green_size              = 0; 
            let mut blue_size               = 0; 
            let mut alpha_size              = 0; 
            let mut depth_size              = 0; 
            let mut stencil_size            = 0; 
            let mut accum_red_size          = 0; 
            let mut accum_green_size        = 0; 
            let mut accum_blue_size         = 0; 
            let mut accum_alpha_size        = 0; 
            let mut render_type             = 0; 
            let mut drawable_type           = 0; 
            let mut x_renderable            = 0; 
            let mut visual_id               = 0; 
            let mut x_visual_type           = 0; 
            let mut config_caveat           = 0; 
            let mut transparent_type        = 0; 
            let mut transparent_index_value = 0; 
            let mut transparent_red_value   = 0; 
            let mut transparent_green_value = 0; 
            let mut transparent_blue_value  = 0; 
            let mut transparent_alpha_value = 0; 
            let mut max_pbuffer_width       = 0; 
            let mut max_pbuffer_height      = 0; 
            let mut max_pbuffer_pixels      = 0; 
            unsafe {
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_SAMPLE_BUFFERS         , &mut sample_buffers         );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_SAMPLES                , &mut samples                );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_FBCONFIG_ID            , &mut fbconfig_id            );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_BUFFER_SIZE            , &mut buffer_size            );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_LEVEL                  , &mut level                  );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_DOUBLEBUFFER           , &mut stereo                 );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_STEREO                 , &mut doublebuffer           );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_AUX_BUFFERS            , &mut aux_buffers            );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_RED_SIZE               , &mut red_size               );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_GREEN_SIZE             , &mut green_size             );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_BLUE_SIZE              , &mut blue_size              );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_ALPHA_SIZE             , &mut alpha_size             );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_DEPTH_SIZE             , &mut depth_size             );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_STENCIL_SIZE           , &mut stencil_size           );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_ACCUM_RED_SIZE         , &mut accum_red_size         );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_ACCUM_GREEN_SIZE       , &mut accum_green_size       );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_ACCUM_BLUE_SIZE        , &mut accum_blue_size        );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_ACCUM_ALPHA_SIZE       , &mut accum_alpha_size       );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_RENDER_TYPE            , &mut render_type            );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_DRAWABLE_TYPE          , &mut drawable_type          );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_X_RENDERABLE           , &mut x_renderable           );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_VISUAL_ID              , &mut visual_id              );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_X_VISUAL_TYPE          , &mut x_visual_type          );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_CONFIG_CAVEAT          , &mut config_caveat          );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_TRANSPARENT_TYPE       , &mut transparent_type       );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_TRANSPARENT_INDEX_VALUE, &mut transparent_index_value);
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_TRANSPARENT_RED_VALUE  , &mut transparent_red_value  );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_TRANSPARENT_GREEN_VALUE, &mut transparent_green_value);
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_TRANSPARENT_BLUE_VALUE , &mut transparent_blue_value );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_TRANSPARENT_ALPHA_VALUE, &mut transparent_alpha_value);
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_MAX_PBUFFER_WIDTH      , &mut max_pbuffer_width      );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_MAX_PBUFFER_HEIGHT     , &mut max_pbuffer_height     );
                glXGetFBConfigAttrib(*x_display, *fbc, GLX_MAX_PBUFFER_PIXELS     , &mut max_pbuffer_pixels     );
            }
            // let visualid = unsafe { (*visual_info).visualid };
            unsafe { 
                x::XFree(visual_info as *mut _);
            }
            let stereo = stereo != x::False;
            let doublebuffer = doublebuffer != x::False;
            let x_renderable = x_renderable != x::False;
            let x_visual_type = match x_visual_type {
                GLX_TRUE_COLOR   => "GLX_TRUE_COLOR",
                GLX_DIRECT_COLOR => "GLX_DIRECT_COLOR",
                GLX_PSEUDO_COLOR => "GLX_PSEUDO_COLOR",
                GLX_STATIC_COLOR => "GLX_STATIC_COLOR",
                GLX_GRAY_SCALE   => "GLX_GRAY_SCALE",
                GLX_STATIC_GRAY  => "GLX_STATIC_GRAY",
                _ => "<??>",
            };
            let config_caveat = match config_caveat {
                GLX_NONE                  => "GLX_NONE",
                GLX_SLOW_CONFIG           => "GLX_SLOW_CONFIG",
                GLX_NON_CONFORMANT_CONFIG => "GLX_NON_CONFORMANT_CONFIG",
                _ => "<??>",
            };
            let transparent_type = match transparent_type {
                GLX_NONE              => "GLX_NONE",
                GLX_TRANSPARENT_RGB   => "GLX_TRANSPARENT_RGB",
                GLX_TRANSPARENT_INDEX => "GLX_TRANSPARENT_INDEX",
                _ => "<??>",
            };

            info!("Matching FBConfig n°{}", i);
            info!("- sample_buffers          : {}", sample_buffers         );
            info!("- samples                 : {}", samples                );
            info!("- fbconfig_id             : 0x{:x}", fbconfig_id            );
            info!("- buffer_size             : {}", buffer_size            );
            info!("- level                   : {}", level                  );
            info!("- stereo                  : {}", stereo                 );
            info!("- doublebuffer            : {}", doublebuffer           );
            info!("- aux_buffers             : {}", aux_buffers            );
            info!("- red_size                : {}", red_size               );
            info!("- green_size              : {}", green_size             );
            info!("- blue_size               : {}", blue_size              );
            info!("- alpha_size              : {}", alpha_size             );
            info!("- depth_size              : {}", depth_size             );
            info!("- stencil_size            : {}", stencil_size           );
            info!("- accum_red_size          : {}", accum_red_size         );
            info!("- accum_green_size        : {}", accum_green_size       );
            info!("- accum_blue_size         : {}", accum_blue_size        );
            info!("- accum_alpha_size        : {}", accum_alpha_size       );
            info!("- render_type             : 0x{:x}{}{}", render_type, 
                if render_type & GLX_RGBA_BIT != 0 { " (GLX_RGBA_BIT)" } else { "" },
                if render_type & GLX_COLOR_INDEX_BIT != 0 { " (GLX_COLOR_INDEX_BIT)" } else { "" }
            );
            info!("- drawable_type           : 0x{:x}{}{}{}", drawable_type,
                if drawable_type & GLX_WINDOW_BIT  != 0 { " (GLX_WINDOW_BIT)"  } else { "" },
                if drawable_type & GLX_PIXMAP_BIT  != 0 { " (GLX_PIXMAP_BIT)"  } else { "" },
                if drawable_type & GLX_PBUFFER_BIT != 0 { " (GLX_PBUFFER_BIT)" } else { "" }
            ); // GLX_WINDOW_BIT, GLX_PIXMAP_BIT, and GLX_PBUFFER_BIT
            info!("- x_renderable            : {}", x_renderable           );
            info!("- visual_id               : 0x{:x}", visual_id              );
            info!("- x_visual_type           : {}", x_visual_type          ); // GLX_TRUE_COLOR, GLX_DIRECT_COLOR, GLX_PSEUDO_COLOR, GLX_STATIC_COLOR, GLX_GRAY_SCALE, or GLX_STATIC_GRAY
            info!("- config_caveat           : {}", config_caveat          ); // GLX_NONE, GLX_SLOW_CONFIG, GLX_NON_CONFORMANT_CONFIG
            info!("- transparent_type        : {}", transparent_type       ); // GLX_NONE, GLX_TRANSPARENT_RGB, GLX_TRANSPARENT_INDEX
            info!("- transparent_index_value : {}", transparent_index_value);
            info!("- transparent_red_value   : {}", transparent_red_value  );
            info!("- transparent_green_value : {}", transparent_green_value);
            info!("- transparent_blue_value  : {}", transparent_blue_value );
            info!("- transparent_alpha_value : {}", transparent_alpha_value);
            info!("- max_pbuffer_width       : {}", max_pbuffer_width      );
            info!("- max_pbuffer_height      : {}", max_pbuffer_height     );
            info!("- max_pbuffer_pixels      : {}", max_pbuffer_pixels     );

            if !is_fbconfig_chosen
            && sample_buffers == settings.msaa.buffer_count as _
            && samples == settings.msaa.sample_count as _
            && doublebuffer == settings.double_buffer as _
            {
                is_fbconfig_chosen = true;
                best_fbc = *fbc;
                best_fbc_i = i;
                // Don't `break`, ensure we run through the whole list first
                // so we can log them all.
            }
        }
        info!("Chosen FBConfig n°{}", best_fbc_i);
        unsafe { 
            let visual_info = glXGetVisualFromFBConfig(*x_display, best_fbc);
            assert!(!visual_info.is_null());
            x::XFree(fbconfigs.as_ptr() as *const _ as *mut _);
            Ok(X11GLPixelFormat { context: Rc::clone(&self.0), visual_info, fbconfig: Some(best_fbc) })
        }
    }
}


impl X11SharedWindow {
    pub fn x11_gl_pixel_format(&self) -> Result<&X11GLPixelFormat> {
        self.x11_gl_pixel_format.as_ref().map_err(Clone::clone)
    }
    pub fn create_gl_context(&self, settings: &GLContextSettings) -> Result<X11GLContext> {
        let glx = self.context.glx()?;
        let x_display = self.context.lock_x_display();
        let &X11GLPixelFormat { visual_info, fbconfig, context: _ } = &self.x11_gl_pixel_format()?;

        let glx_lt_1_3 = version_cmp::lt((glx.major_version, glx.minor_version), (1, 3));
        let glx_lt_1_4 = version_cmp::lt((glx.major_version, glx.minor_version), (1, 4));

        let (f, glx_context) = unsafe {
            let get_glx_context = || if glx_lt_1_3 {
                ("glXCreateContext", glXCreateContext(*x_display, *visual_info, ptr::null_mut(), x::True))
            } else if glx_lt_1_4 || (!glx_lt_1_4 && !glx.ext.GLX_ARB_create_context) {
                ("glXCreateNewContext", glXCreateNewContext(*x_display, fbconfig.unwrap(), GLX_RGBA_TYPE, ptr::null_mut(), x::True))
            } else {
                let f = glx.ext.glXCreateContextAttribsARB.unwrap();
                let attribs_arb = glx.gen_arb_attribs(settings);
                ("glXCreateContextAttribsARB", (f)(*x_display, fbconfig.unwrap(), ptr::null_mut(), x::True, attribs_arb.as_ptr()))
            };

            xlib_error::sync_catch(*x_display, get_glx_context)?
        };
        if glx_context.is_null() {
            return failed(format!("{}() returned NULL", f));
        }
        Ok(X11GLContext { context: Rc::clone(&self.context), glx_context })
    }

    pub fn make_gl_context_current(&self, c: Option<&X11GLContext>) -> Result<()> {
        let x_display = self.context.lock_x_display();
        let glx_context = match c {
            Some(c) => c.glx_context,
            None => ptr::null_mut(),
        };
        unsafe {
            match self.glx_window {
                Some(w) => glXMakeContextCurrent(*x_display, w, w, glx_context),
                None => glXMakeCurrent(*x_display, self.x_window, glx_context),
            };
        }
        Ok(())
    }

    pub fn gl_swap_buffers(&self) -> Result<()> {
        unsafe {
            glXSwapBuffers(*self.context.lock_x_display(), match self.glx_window {
                Some(w) => w,
                None => self.x_window,
            });
        }
        Ok(())
    }
    pub fn gl_set_swap_interval(&self, interval: GLSwapInterval) -> Result<()> {
        let glx = self.context.glx()?;

        let interval = match interval {
            GLSwapInterval::VSync => 1,
            GLSwapInterval::Immediate => 0,
            GLSwapInterval::LateSwapTearing => {
                if !glx.ext.GLX_EXT_swap_control_tear {
                    return failed("Missing extension `GLX_EXT_swap_control_tear`");
                }
                -1
            },
            GLSwapInterval::Interval(i) => {
                if i < 0 && !glx.ext.GLX_EXT_swap_control_tear {
                    return failed("Missing extension `GLX_EXT_swap_control_tear`");
                }
                i
            },
        };

        if glx.ext.GLX_EXT_swap_control && self.glx_window.is_some() {
            let ssi = glx.ext.glXSwapIntervalEXT.unwrap();
            unsafe {
                ssi(*self.context.lock_x_display(), self.glx_window.unwrap(), interval);
            }
            Ok(())
        } else if glx.ext.GLX_MESA_swap_control {
            let ssi = glx.ext.glXSwapIntervalMESA.unwrap();
            unsafe {
                ssi(interval);
            }
            Ok(())
        } else if glx.ext.GLX_SGI_swap_control {
            let ssi = glx.ext.glXSwapIntervalSGI.unwrap();
            unsafe {
                ssi(interval);
            }
            Ok(())
        } else {
            failed("There's no extension that could set the swap interval!")
        }
    }
}
