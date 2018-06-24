use std::ffi::{CStr, CString};
use std::os::raw::c_int;
use std::mem;
use super::x11::xlib as x;
use super::x11::glx::*;
use super::x11::glx::arb::*;
use super::missing_bits::glx::*;
use super::X11SharedContext;
use gl::{GLPixelFormatSettings, GLContextSettings, GLVariant, GLContextResetNotificationStrategy, GLVersion, GLProfile};
use error::{Result, failed};
use version_cmp;

pub mod fn_types {
    #![allow(non_camel_case_types)]

    use ::std::os::raw::c_int;
    use super::super::x11::xlib as x;
    use super::super::x11::glx::*;

    pub type glXGetProcAddress = unsafe extern fn(*const u8) -> Option<unsafe extern fn()>;
    pub type glXSwapIntervalMESA = unsafe extern fn(interval: c_int) -> c_int;
    pub type glXGetSwapIntervalMESA = unsafe extern fn() -> c_int;
    pub type glXSwapIntervalSGI = unsafe extern fn(interval: c_int) -> c_int;
    pub type glXSwapIntervalEXT = unsafe extern fn(
        *mut x::Display, GLXDrawable, interval: c_int
    );
    pub type glXCreateContextAttribsARB = unsafe extern fn(
        *mut x::Display, GLXFBConfig, share_context: GLXContext, 
        direct: x::Bool, attrib_list: *const c_int
    ) -> GLXContext;
}


macro_rules! glx_ext {
    (($($name:ident)+) ($($func:ident)+)) => {
        #[allow(non_snake_case)]
        #[derive(Debug, Copy, Clone, Default, Hash, PartialEq, Eq)]
        pub struct GlxExt {
            $(pub $name: bool,)+
            $(pub $func: Option<fn_types::$func>,)+
        }
        impl GlxExt {
            #[allow(non_snake_case)]
            pub fn parse(gpa: fn_types::glXGetProcAddress, s: &CStr) -> Self {
                $(let mut $name = false;)+
                let s = s.to_string_lossy();
                for name in s.split_whitespace() {
                    match name {
                        $(stringify!($name) => {
                            $name = true;
                            info!("Found GLX extension {}", stringify!($name));
                        },)+
                        _ => {}
                    };
                }
                let mut out = Self { $($name,)+ $($func: None,)+ };

                // Load functions
                unsafe { $(
                    let cstring = CString::new(stringify!($func)).unwrap_or_default();
                    let name = cstring.to_bytes_with_nul();
                    let fptr = gpa(name.as_ptr() as *mut _);
                    out.$func = match fptr {
                        None => {
                            warn!("Couldn't load `{}`", stringify!($func));
                            None
                        },
                        Some(f) => {
                            info!("Loaded `{}`", stringify!($func));
                            Some(::std::mem::transmute(f))
                        },
                    };
                )+ }

                out
            }
        }
    }
}


glx_ext!((
    GLX_ARB_multisample
    GLX_EXT_swap_control
    GLX_EXT_swap_control_tear
    GLX_MESA_swap_control
    GLX_SGI_swap_control
    GLX_SGI_video_sync
    GLX_OML_swap_method
    GLX_OML_sync_control
    GLX_ARB_create_context
    GLX_ARB_create_context_profile
    GLX_ARB_create_context_robustness
    GLX_EXT_create_context_es_profile
    GLX_EXT_create_context_es2_profile
    )(
    glXSwapIntervalEXT
    glXSwapIntervalMESA
    glXGetSwapIntervalMESA
    glXSwapIntervalSGI
    glXCreateContextAttribsARB
));



#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Glx {
    pub event_base: c_int,
    pub error_base: c_int,
    pub major_version: c_int,
    pub minor_version: c_int,

    pub get_proc_address: fn_types::glXGetProcAddress,
    pub ext: GlxExt,
}

impl X11SharedContext {
    pub fn glx(&self) -> Result<&Glx> {
        self.glx.as_ref().map_err(Clone::clone)
    }
}

impl Glx {
    // Functions that generate context attrib arrays (i.e 0-terminated
    // arrays of i32).
    //
    // gen_visual_attribs() and gen_fbconfig_attribs() are two separate 
    // functions for ease of maintenance. They don't have all keys in
    // common, and the format is different.
    // For instance, GLX_DOUBLEBUFFER and GLX_STEREO are not followed by
    // a boolean in gen_visual_attribs() - their presence _is_ the boolean
    // instead.

    // GLX below 1.3
    pub fn gen_visual_attribs(&self, settings: &GLPixelFormatSettings) -> [c_int; 30] {
        let &GLPixelFormatSettings {
            depth_bits, stencil_bits, double_buffer, stereo,
            red_bits, blue_bits, green_bits, alpha_bits,
            accum_red_bits, accum_blue_bits, accum_green_bits, 
            accum_alpha_bits, aux_buffers, msaa,
        } = settings;
        let mut attr = [
            GLX_RGBA,
            GLX_AUX_BUFFERS, aux_buffers as c_int,
            GLX_RED_SIZE, red_bits as c_int,
            GLX_GREEN_SIZE, green_bits as c_int,
            GLX_BLUE_SIZE, blue_bits as c_int,
            GLX_ALPHA_SIZE, alpha_bits as c_int,
            GLX_DEPTH_SIZE, depth_bits as c_int,
            GLX_STENCIL_SIZE, stencil_bits as c_int,
            GLX_ACCUM_RED_SIZE, accum_red_bits as c_int,
            GLX_ACCUM_GREEN_SIZE, accum_green_bits as c_int,
            GLX_ACCUM_BLUE_SIZE, accum_blue_bits as c_int,
            GLX_ACCUM_ALPHA_SIZE, accum_alpha_bits as c_int,
            0, // GLX_DOUBLEBUFFER, see below
            0, // GLX_STEREO, see below
            0, // GLX_SAMPLE_BUFFERS_ARB attrib, see below
            0, // GLX_SAMPLE_BUFFERS_ARB value, see below
            0, // GLX_SAMPLES_ARB attrib, see below
            0, // GLX_SAMPLES_ARB value, see below
            0 // end
        ];

        let mut i = attr.len()-7;
        if double_buffer {
            attr[i] = GLX_DOUBLEBUFFER;
            i += 1;
        }
        if stereo {
            attr[i] = GLX_STEREO;
            i += 1;
        }
        if self.ext.GLX_ARB_multisample {
            attr[i+0] = GLX_SAMPLE_BUFFERS; // Same as prefixed with _ARB
            attr[i+1] = msaa.buffer_count as _;
            attr[i+2] = GLX_SAMPLES; // Same as prefixed with _ARB
            attr[i+3] = msaa.sample_count as _;
        }
        attr
    }

    // GLX 1.3 and above
    pub fn gen_fbconfig_attribs(&self, settings: &GLPixelFormatSettings) -> [c_int; 43] {
        let &GLPixelFormatSettings {
            depth_bits, stencil_bits, double_buffer, stereo,
            red_bits, blue_bits, green_bits, alpha_bits,
            accum_red_bits, accum_blue_bits, accum_green_bits, 
            accum_alpha_bits, aux_buffers, msaa, ..
        } = settings;
        let mut attribs = [
            GLX_FBCONFIG_ID, GLX_DONT_CARE,
            GLX_DOUBLEBUFFER, double_buffer as c_int,
            GLX_STEREO, stereo as c_int,
            GLX_AUX_BUFFERS, aux_buffers as c_int,
            GLX_RED_SIZE, red_bits as c_int,
            GLX_GREEN_SIZE, green_bits as c_int,
            GLX_BLUE_SIZE, blue_bits as c_int,
            GLX_ALPHA_SIZE, alpha_bits as c_int,
            GLX_DEPTH_SIZE, depth_bits as c_int,
            GLX_STENCIL_SIZE, stencil_bits as c_int,
            GLX_ACCUM_RED_SIZE, accum_red_bits as c_int,
            GLX_ACCUM_GREEN_SIZE, accum_green_bits as c_int,
            GLX_ACCUM_BLUE_SIZE, accum_blue_bits as c_int,
            GLX_ACCUM_ALPHA_SIZE, accum_alpha_bits as c_int,
            GLX_RENDER_TYPE, GLX_RGBA_BIT,
            GLX_DRAWABLE_TYPE, GLX_WINDOW_BIT,
            GLX_X_VISUAL_TYPE, GLX_TRUE_COLOR,
            GLX_X_RENDERABLE, x::True,
            GLX_CONFIG_CAVEAT, GLX_DONT_CARE, // NOTE: Setting it to GLX_NONE is very strict.
            0, 0, // GLX_SAMPLE_BUFFERS, msaa.buffer_count as _, // FIXME: Nobody said we had GLX_ARB_MULTISAMPLE!
            0, 0, // GLX_SAMPLES, msaa.sample_count as _,
            0 // keep last
        ];
        let mut i = attribs.len() - 5;
        assert_eq!(0, attribs[i]);
        if self.ext.GLX_ARB_multisample {
            assert_eq!(GLX_SAMPLE_BUFFERS, GLX_SAMPLE_BUFFERS_ARB);
            assert_eq!(GLX_SAMPLES, GLX_SAMPLES_ARB);
            attribs[i] = GLX_SAMPLE_BUFFERS;
            i += 1;
            attribs[i] = msaa.buffer_count as _;
            i += 1;
            attribs[i] = GLX_SAMPLES;
            i += 1;
            attribs[i] = msaa.sample_count as _;
            i += 1;
        }
        assert_eq!(0, *attribs.last().unwrap());
        attribs
    }

    // Configure an array of attribute parameters for 
    // glxCreateContextAttribsARB().
    pub fn gen_arb_attribs(&self, settings: &GLContextSettings) -> [c_int; 11] {

        let &GLContextSettings {
            version, robust_access, debug, forward_compatible, profile, ..
        } = settings;

        #[allow(non_snake_case)]
        let &GlxExt {
            GLX_ARB_create_context_profile,
            GLX_ARB_create_context_robustness,
            GLX_EXT_create_context_es_profile,
            ..
        } = &self.ext;

        let (major, minor, gl_variant) = match version {
            Some(GLVersion { major, minor, variant }) => (major, minor, variant),
            None => (3, 0, GLVariant::Desktop),
        };

        let flags = if debug { 
            GLX_CONTEXT_DEBUG_BIT_ARB
        } else { 0 }
        | if forward_compatible {
            GLX_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB
        } else { 0 }
        | if robust_access.is_some() && GLX_ARB_create_context_robustness {
            GLX_CONTEXT_ROBUST_ACCESS_BIT_ARB
        } else { 0 };

        let profile_param = match gl_variant {
            GLVariant::Desktop if GLX_ARB_create_context_profile =>
                GLX_CONTEXT_PROFILE_MASK_ARB,
            GLVariant::ES if GLX_EXT_create_context_es_profile =>
                GLX_CONTEXT_PROFILE_MASK_ARB,
            _ => 0,
        };

        let profile_mask = match gl_variant {
            GLVariant::Desktop => match profile {
                None => GLX_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
                Some(p) => match p {
                    GLProfile::Core =>
                        GLX_CONTEXT_CORE_PROFILE_BIT_ARB,
                    GLProfile::Compatibility => 
                        GLX_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
                }
            },
            GLVariant::ES =>
                // Same as GLX_CONTEXT_ES2_PROFILE_BIT_EXT.
                GLX_CONTEXT_ES_PROFILE_BIT_EXT,
        };

        let robust_param = if robust_access.is_some() {
            GLX_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB
        } else { 0 };

        let robust_value = match robust_access {
            None => 0,
            Some(GLContextResetNotificationStrategy::NoResetNotification) =>
                    GLX_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB,
            Some(GLContextResetNotificationStrategy::LoseContextOnReset) =>
                    GLX_LOSE_CONTEXT_ON_RESET_ARB,
        };

        let mut out = [
            GLX_CONTEXT_MAJOR_VERSION_ARB, major as _,
            GLX_CONTEXT_MINOR_VERSION_ARB, minor as _,
            GLX_CONTEXT_FLAGS_ARB, flags,
            0 /* profile_param */, 0 /* profile_mask */,
            0 /* robust_param */, 0 /* robust_value */,
            0
        ];

        let mut i = out.len()-5;
        if profile_param != 0 {
            out[i] = profile_param;
            out[i+1] = profile_mask;
            i += 2;
        }
        if robust_param != 0 {
            out[i] = robust_param;
            out[i+1] = robust_value;
        }

        out
    }

    pub unsafe fn query(x_display: *mut x::Display) -> Result<Self> {
        let (mut error_base, mut event_base) = mem::uninitialized();
        let has_glx = glXQueryExtension(x_display, &mut error_base, &mut event_base);
        if has_glx == x::False {
            return failed("GLX extension is missing");
        }

        let (mut major, mut minor) = mem::uninitialized();
        let success = glXQueryVersion(x_display, &mut major, &mut minor);
        if success == x::False {
            return failed("glXQueryVersion() failed");
        }

        #[cfg(not(target_os = "linux"))]
        unimplemented!("We don't know how the situation is in OSes other than Linux! This could require moving to x11-dl.");
        #[cfg(target_os = "linux")]
        let get_proc_address = glXGetProcAddressARB;

        let mut glx = Glx {
            error_base, 
            event_base,
            major_version: major,
            minor_version: minor,
            get_proc_address,
            ext: Default::default(),
        };

        if version_cmp::lt((major, minor), (1, 1)) {
            warn!("The GLX version is less than 1.1! This is supposedly very rare and probably badly handled. Sorry!");
            return Ok(glx);
        }

        glx.ext = unsafe {
            let screen_num = x::XDefaultScreen(x_display);
            let client_vendor  = glXGetClientString(  x_display, GLX_VENDOR);
            let client_version = glXGetClientString(  x_display, GLX_VERSION);
            let server_vendor  = glXQueryServerString(x_display, screen_num, GLX_VENDOR);
            let server_version = glXQueryServerString(x_display, screen_num, GLX_VERSION);
            let extensions = glXQueryExtensionsString(x_display, screen_num);
            info!("GLX client vendor : {:?}", CStr::from_ptr(client_vendor ).to_str());
            info!("GLX client version: {:?}", CStr::from_ptr(client_version).to_str());
            info!("GLX server vendor : {:?}", CStr::from_ptr(server_vendor ).to_str());
            info!("GLX server version: {:?}", CStr::from_ptr(server_version).to_str());
            info!("GLX extensions    : {:?}", CStr::from_ptr(extensions    ).to_str());
            GlxExt::parse(get_proc_address, &CStr::from_ptr(extensions))
        };

        Ok(glx)
    }
}

