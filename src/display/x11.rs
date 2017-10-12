// Useful links and resources :
// - Well-written docs for X11
//   https://tronche.com/gui/x/
// - Extended Window Manager Hints
//   https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html
// - Xplain on the XComposite extension
//   https://magcius.github.io/xplain/article/composite.html
//   https://cgit.freedesktop.org/xorg/proto/compositeproto/tree/compositeproto.txt
// - Translucent Windows in X
//   https://keithp.com/~keithp/talks/KeithPackardAls2000/
// - Clipboard wiki
//   https://www.freedesktop.org/wiki/Specifications/ClipboardsWiki/
// - XDND
//   https://www.freedesktop.org/wiki/Specifications/XDND/
// - GLX 1.4 spec
//   https://www.khronos.org/registry/OpenGL/specs/gl/glx1.4.pdf
// - GLX extensions
//   https://www.khronos.org/registry/OpenGL/index_gl.php
//
// Missing features:
// - Copy contexts
// - Share contexts (at glXCreateContext)
// - Off-screen rendering
//
// TODO: Talk a bit about how X11 PreparedAtoms are like Keys or Values for 
// Properties.
//
// The plan:
// - Display::open() chooses a screen (for use other than XDefaultScreen)
// - Get the root window for the screen with XRootWindow;
// - Display::open() calls glXQueryExtension and glXQueryVersion.
// - Display::open() calls glXGetClientString and glXQueryServerString
//   and glXQueryExtensionsString;
//
// Then, depending one the GLX version:
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
// Then :
// - Log glXIsDirect()


extern crate x11;
use self::x11::xlib as x;
use self::x11::glx::*;
use self::x11::glx::arb::*;
use std::fmt::{self, Formatter};
use std::ptr;
use std::mem;
use std::ffi::*;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};

use super::Extent2;

use Semver;

use super::*;
use super::window::{
    Capabilities,
    WindowOpResult,
};


mod types {
    #![allow(non_camel_case_types)]
    use super::*;
    pub type glXGetProcAddress = unsafe extern fn(*const u8) -> Option<unsafe extern fn()>;
    pub type glXSwapIntervalMESA = unsafe extern fn(interval: c_int) -> c_int;
    pub type glXGetSwapIntervalMESA = unsafe extern fn() -> c_int;
    pub type glXSwapIntervalSGI = unsafe extern fn(interval: c_int) -> c_int;
    pub type glXSwapIntervalEXT = unsafe extern fn(
        *mut x::Display, GLXDrawable, interval: c_int
    );
    pub type glXCreateContextAttribsARB = unsafe extern fn(
        *mut x::Display, GLXFBConfig, share_context: GLXContext, 
        direct: x::Bool, attrib_list: *const c_int) -> GLXContext;
}


// NOTE: Most structs and fields are not pub(super) because we want them to
// be accessible from the outside world if they opt-in.
// Users do have to use `get_internal()` on their window already.

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Display {
    pub x_dpy: *mut x::Display,
    pub atoms: PreparedAtoms,
    pub screen: *mut x::Screen, // NOTE: Nothing says it needs to be freed, so we don't.
    pub screen_num: c_int,
    pub root: x::Window,
    pub glx: Option<Glx>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Glx {
    pub version: Semver,
    pub get_proc_address: types::glXGetProcAddress,
    pub ext: GlxExt,
    pub error_base: c_int,
    pub event_base: c_int,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Window<'dpy> {
    pub dpy: &'dpy Display,
    pub x_window: x::Window,
    pub colormap: x::Colormap,
    pub glx_window: Option<GLXWindow>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub(super) struct GLContext<'dpy> {
    pub dpy: &'dpy Display,
    pub glx_context: GLXContext,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct GLPixelFormat<'dpy> {
    pub dpy: &'dpy Display,
    pub visual_info: *mut x::XVisualInfo,
    pub fbconfig: Option<GLXFBConfig>, // GLX >= 1.3
}

#[derive(Debug, Clone)]
pub enum Error {
    NoXDisplayForName { name: Option<CString> },
    NoGLX,
    UnsupportedGLContextSettings,
    FunctionName(&'static str),
}

// TODO
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Cursor<'dpy> {
    pub dpy: &'dpy Display,
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            x::XCloseDisplay(self.x_dpy);
        } 
    }
}

impl<'dpy> Drop for Window<'dpy> {
    fn drop(&mut self) {
        unsafe {
            match self.glx_window {
                Some(w) => glXDestroyWindow(self.dpy.x_dpy, w),
                None => (),
            };
            x::XDestroyWindow(self.dpy.x_dpy, self.x_window);
            x::XFreeColormap(self.dpy.x_dpy, self.colormap);
        }
    }
}

impl<'dpy> Drop for GLPixelFormat<'dpy> {
    fn drop(&mut self) {
        unsafe {
            x::XFree(self.visual_info as *mut _); // NOTE: Fine to do on NULL.
        }
    }
}

impl<'dpy> Drop for GLContext<'dpy> {
    fn drop(&mut self) {
        unsafe {
            // Defers destruction until it's not current to any thread.
            glXDestroyContext(self.dpy.x_dpy, self.glx_context);
        }
    }
}





impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match self {
            &Error::NoXDisplayForName { ref name } => {
                let name = unsafe { CString::from_raw(x::XDisplayName(
                    match name {
                        &None => ptr::null(),
                        &Some(ref name) => &name as *const _ as *const i8,
                    }
                ))};
                let name = name.to_str().unwrap_or("<utf-8 conversion error>");
                write!(fmt, "\"{}\" is not a valid X display", name)
            },
            &Error::NoGLX => {
                write!(fmt, "The GLX extension is not present")
            },
            &Error::UnsupportedGLContextSettings => {
                write!(fmt, "Unsupported OpenGL context settings")
            },
            &Error::FunctionName(name) => {
                write!(fmt, "{}() failed", name)
            },
        }
    }
}


/// Generate this module's `PreparedAtoms` struct, where all atoms are retrieved
/// once when opening a display.
macro_rules! atoms {
    ($($atom:ident)+) => {
        #[allow(non_snake_case)]
        #[derive(Debug, Hash, PartialEq, Eq)]
        pub struct PreparedAtoms {
            $(pub $atom: x::Atom,)+
        }
        impl PreparedAtoms {
            fn fetch(x_dpy: *mut x::Display) -> Self {
                Self { $(
                    $atom: match unsafe { x::XInternAtom(x_dpy, 
                        // PERF: Worried about CString
                        CString::new(stringify!($atom)).unwrap().as_ptr(), 
                        x::False)
                    } {
                        0 => 0,
                        atom @ _ => {
                            info!("Found atom {} = {}", stringify!($atom), atom);
                            atom
                        }
                    },
                )+ }
            }
        }
    }
}

atoms!(
    WM_PROTOCOLS
    WM_DELETE_WINDOW
    WM_TAKE_FOCUS
    _NET_WORKAREA
    _NET_NUMBER_OF_DESKTOPS
    _NET_CURRENT_DESKTOP
    _NET_DESKTOP_NAMES
    _NET_DESKTOP_VIEWPORT
    _NET_DESKTOP_GEOMETRY
    _NET_ACTIVE_WINDOW
    _NET_WM_NAME
    _NET_WM_ICON_NAME
    _NET_WM_WINDOW_TYPE
    _NET_WM_WINDOW_TYPE_DESKTOP
    _NET_WM_WINDOW_TYPE_SPLASH
    _NET_WM_WINDOW_TYPE_DIALOG
    _NET_WM_WINDOW_TYPE_NOTIFICATION
    // and others.. ??

    _NET_WM_STATE
    _NET_WM_STATE_HIDDEN
    _NET_WM_STATE_FOCUSED
    _NET_WM_STATE_MAXIMIZED_VERT
    _NET_WM_STATE_MAXIMIZED_HORZ
    _NET_WM_STATE_FULLSCREEN
    _NET_WM_STATE_ABOVE
    _NET_WM_STATE_SKIP_TASKBAR
    _NET_WM_STATE_SKIP_PAGER
    _NET_WM_STATE_DEMANDS_ATTENTION

    _NET_WM_ALLOWED_ACTIONS
    _NET_WM_ACTION_FULLSCREEN

    // This is an array of 32bit packed CARDINAL ARGB with high byte being A, low byte being B. The first two cardinals are width, height. Data is in rows, left to right and top to bottom.
    _NET_WM_ICON

    _NET_WM_PID

    // Should set this when going off-screen.
    _NET_WM_BYPASS_COMPOSITOR

    _NET_FRAME_EXTENTS
    _NET_WM_PING
    _NET_WM_WINDOW_OPACITY // Doesn't seem to be defined officially ??
    UTF8_STRING
    PRIMARY

    // X drag'n Drop atoms
    XdndEnter
    XdndPosition
    XdndStatus
    XdndTypeList
    XdndActionCopy
    XdndDrop
    XdndFinished
    XdndSelection

    // ??? from SDL2
    XKLAVIER_STATE
);


macro_rules! glx_ext {
    (($($name:ident)+) ($($func:ident)+)) => {
        #[allow(non_snake_case)]
        #[derive(Debug, Copy, Clone, Default, Hash, PartialEq, Eq)]
        pub struct GlxExt {
            $(pub $name: bool,)+
            $(pub $func: Option<types::$func>,)+
        }
        impl GlxExt {
            #[allow(non_snake_case)]
            pub fn parse(gpa: types::glXGetProcAddress, s: &CStr) -> Self {
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
                        None => None,
                        Some(f) => {
                            info!("Loaded `{}`", stringify!($func));
                            Some(mem::transmute(f))
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
    glXSwapIntervalMESA
    glXGetSwapIntervalMESA
    glXSwapIntervalSGI
    glXSwapIntervalEXT
    glXCreateContextAttribsARB
));


// TODO: Send a PR to x11-rs.
mod xx {
    pub const GLX_CONTEXT_ES_PROFILE_BIT_EXT             : i32 = 0x00000004;
    pub const GLX_CONTEXT_ES2_PROFILE_BIT_EXT            : i32 = 0x00000004;
    pub const GLX_CONTEXT_ROBUST_ACCESS_BIT_ARB          : i32 = 0x00000004;
    pub const GLX_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB: i32 = 0x8256;
    pub const GLX_NO_RESET_NOTIFICATION_ARB              : i32 = 0x8261;
    pub const GLX_LOSE_CONTEXT_ON_RESET_ARB              : i32 = 0x8252;
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
    fn gen_visual_attribs(&self, settings: &GLPixelFormatSettings) -> [c_int; 30] {
        let &GLPixelFormatSettings {
            depth_bits, stencil_bits, double_buffer, stereo,
            red_bits, blue_bits, green_bits, alpha_bits,
            accum_red_bits, accum_blue_bits, accum_green_bits, 
            accum_alpha_bits, aux_buffers, msaa, ..
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
            0, // GLX_SAMPLE_BUFFERS_ARB, see below
            0, // GLX_SAMPLE_BUFFERS_ARB value, see below
            0, // GLX_SAMPLES_ARB, see below
            0, // GLX_SAMPLES_ARB value, see below
            0
        ];
        // GLX_ARB_multisample
        // GLX_SAMPLE_BUFFERS, msaa.buffer_count,
        // GLX_SAMPLES, msaa.sample_count,

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
    fn gen_fbconfig_attribs(&self, settings: &GLPixelFormatSettings) -> [c_int; 41] {
        let &GLPixelFormatSettings {
            depth_bits, stencil_bits, double_buffer, stereo,
            red_bits, blue_bits, green_bits, alpha_bits,
            accum_red_bits, accum_blue_bits, accum_green_bits, 
            accum_alpha_bits, aux_buffers, msaa, ..
        } = settings;
        [
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
            GLX_SAMPLE_BUFFERS, msaa.buffer_count as _,
            GLX_SAMPLES, msaa.sample_count as _,
            // GLX_CONFIG_CAVEAT, 0,
            //
            // There's more GLX_TRANSPARENT_**_VALUE keys, might be
            // worth checking later,
            0 // keep last
        ]
    }

    // Configure an array of attribute parameters for 
    // glxCreateContextAttribsARB().
    fn gen_arb_attribs(&self, settings: &GLContextSettings) -> [c_int; 11] {

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
            Decision::Manual(v) => {
                let v = v.to_semver();
                (v.1.major, v.1.minor, v.0)
            },
            Decision::Auto => (3, 0, GLVariant::Desktop), // TODO: Shouldn't it be 3.2 ?
        };

        let flags = if debug { 
            GLX_CONTEXT_DEBUG_BIT_ARB
        } else { 0 }
        | if forward_compatible {
            GLX_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB
        } else { 0 }
        | if robust_access.is_some() && GLX_ARB_create_context_robustness {
            xx::GLX_CONTEXT_ROBUST_ACCESS_BIT_ARB
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
                Decision::Auto => GLX_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
                Decision::Manual(p) => match p {
                    GLProfile::Core =>
                        GLX_CONTEXT_CORE_PROFILE_BIT_ARB,
                    GLProfile::Compatibility => 
                        GLX_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
                }
            },
            GLVariant::ES =>
                // Same as GLX_CONTEXT_ES2_PROFILE_BIT_EXT.
                xx::GLX_CONTEXT_ES_PROFILE_BIT_EXT,
        };

        let robust_param = if robust_access.is_some() {
            xx::GLX_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB
        } else { 0 };

        let robust_value = match robust_access {
            None => 0,
            Some(r) => match r.context_reset_notification_strategy {
                GLContextResetNotificationStrategy::NoResetNotification =>
                    xx::GLX_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB,
                GLContextResetNotificationStrategy::LoseContextOnReset =>
                    xx::GLX_LOSE_CONTEXT_ON_RESET_ARB,
            },
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
}

static mut G_XLIB_ERROR_OCCURED: AtomicBool = ATOMIC_BOOL_INIT;
static mut G_GLX_ERROR_BASE: i32 = 0;
static mut G_GLX_EVENT_BASE: i32 = 0;

impl Display {

    // TODO: Grab from _XPrintDefaultError in Xlib's sources
    unsafe extern fn x_generic_error_handler(dpy: *mut x::Display, e: *mut x::XErrorEvent) -> c_int {
        // NOTE: DO NOT make requests to the X server within X error handlers such as this one.
        G_XLIB_ERROR_OCCURED.store(true, Ordering::SeqCst);
        let e = *e;
        error!("Received X error: XErrorEvent {{ type: {}, display: {:?}, resourceid: {}, serial: {}, error_code: {}, request_code: {}, minor_code: {} }}", e.type_, e.display, e.resourceid, e.serial, e.error_code, e.request_code, e.minor_code);
        0
    }

    pub(super) fn open() -> Result<Self, super::Error> {
        Self::open_x11_display_name(None)
    }

    pub(super) fn open_x11_display_name(x_display_name: Option<&CStr>) 
        -> Result<Self, super::Error> 
    {
        unsafe {
            // This thing is global to Xlib, and not inherent to X11.
            // We wouldn't have it if we used XCB.
            // info!("Overriding process-wide Xlib error handler.");
            // x::XSetErrorHandler(Some(Self::x_generic_error_handler));

            let x_dpy = x::XOpenDisplay(match x_display_name {
                Some(s) => {
                    info!("Opening X display {}", s.to_string_lossy());
                    s.as_ptr()
                },
                None => {
                    info!("Opening default X display");
                    ptr::null()
                }
            });
            if x_dpy.is_null() {
                return Err(super::Error::Backend(
                    // PERF: No thrilled about this allocation though
                    Error::NoXDisplayForName { 
                        name: x_display_name.map(|s| CString::new(
                            s.to_bytes_with_nul().to_owned()
                        ).unwrap_or_default())
                    }
                ));
            }

            // TODO: Log a lot of X-server-related info, such as the
            // X extensions it supports.

            let screen = x::XDefaultScreenOfDisplay(x_dpy);
            let screen_num = x::XDefaultScreen(x_dpy);
            let root = x::XRootWindowOfScreen(screen);
            let atoms = PreparedAtoms::fetch(x_dpy);
            let glx = Self::query_glx(x_dpy, screen_num);

            Ok(Self { x_dpy, atoms, screen, screen_num, root, glx })
        }
    }

    fn query_glx(x_dpy: *mut x::Display, screen_num: c_int) -> Option<Glx> {

        let (error_base, event_base) = unsafe {
            let (mut error_base, mut event_base) = mem::uninitialized();
            let has_glx = glXQueryExtension(x_dpy, &mut error_base, &mut event_base);
            if has_glx == x::False {
                return None;
            }
            (error_base, event_base)
        };
        unsafe {
            G_GLX_ERROR_BASE = error_base;
            G_GLX_EVENT_BASE = event_base;
        }
        info!("GLX error_base = {}, event_base = {}", error_base, event_base);

        let (major, minor) = unsafe {
            let (mut major, mut minor) = mem::uninitialized();
            let success = glXQueryVersion(x_dpy, &mut major, &mut minor);
            if success == x::False {
               return None;
            }
            (major as u32, minor as u32)
        };
        let version = Semver::new(major, minor, 0);

        info!("GLX extension version {}.{}", major, minor);

        #[cfg(not(target_os = "linux"))]
        unimplemented!("We don't know how the situation is in OSes other than Linux!");
        #[cfg(target_os = "linux")]
        let get_proc_address = glXGetProcAddressARB;

        if version < Semver::new(1,1,0) {
            warn!("The GLX version is less than 1.1! This is supposedly very rare and probably badly handled. Sorry!");
            return Some(Glx {
                version, get_proc_address, ext: Default::default(),
                error_base, event_base,
            });
        }

        let ext = unsafe {
            let client_vendor  = glXGetClientString(  x_dpy, GLX_VENDOR);
            let client_version = glXGetClientString(  x_dpy, GLX_VERSION);
            let server_vendor  = glXQueryServerString(x_dpy, screen_num, GLX_VENDOR);
            let server_version = glXQueryServerString(x_dpy, screen_num, GLX_VERSION);
            let extensions = glXQueryExtensionsString(x_dpy, screen_num);
            info!("GLX client vendor : {:?}", CStr::from_ptr(client_vendor ).to_str());
            info!("GLX client version: {:?}", CStr::from_ptr(client_version).to_str());
            info!("GLX server vendor : {:?}", CStr::from_ptr(server_vendor ).to_str());
            info!("GLX server version: {:?}", CStr::from_ptr(server_version).to_str());
            info!("GLX extensions    : {:?}", CStr::from_ptr(extensions    ).to_str());
            GlxExt::parse(get_proc_address, &CStr::from_ptr(extensions))
        };
        Some(Glx { version, get_proc_address, ext, error_base, event_base })
    }

    pub(super) fn choose_gl_pixel_format<'dpy>(&'dpy self, settings: &GLPixelFormatSettings)
        -> Result<GLPixelFormat<'dpy>, super::Error>
    {
        let x_dpy = self.x_dpy;

        if self.glx.is_none() {
            return Err(super::Error::Backend(Error::NoGLX));
        }
        let glx = self.glx.as_ref().unwrap();

        if glx.version < Semver::new(1,3,0) {
            // Not actually mutated, but glXChooseVisual wants *mut...
            let mut visual_attribs = glx.gen_visual_attribs(settings);
            let visual_info = unsafe { glXChooseVisual(
                x_dpy, self.screen_num, visual_attribs.as_mut_ptr()
            )};
            if visual_info.is_null() {
                return Err(super::Error::Backend(Error::UnsupportedGLContextSettings));
            }
            return Ok(GLPixelFormat { dpy: self, visual_info, fbconfig: None });
        }

        // If we're here, we have GLX >= 1.3.

        let visual_attribs = glx.gen_fbconfig_attribs(settings);
        let mut fbcount: c_int = 0;
        let fbcs = unsafe { glXChooseFBConfig(
            x_dpy, self.screen_num, visual_attribs.as_ptr(), &mut fbcount
        )};
        if fbcs.is_null() || fbcount == 0 {
            warn!("No matching FBConfigs were found!");
            return Err(super::Error::Backend(Error::UnsupportedGLContextSettings));
        }

        // fbcs is an array of candidates, from which we choose the best.

        let mut best_fbc = unsafe { *fbcs };
        let mut best_sample_count: i32 = 0;

        for i in 0..fbcount {
            let fbc = unsafe { *fbcs.offset(i as isize) };
            let visual_info = unsafe {
                glXGetVisualFromFBConfig(x_dpy, fbc)
            };
            if visual_info.is_null() {
                continue;
            }
            let (mut samp_buf, mut samples): (c_int, c_int) = (0, 0);
            unsafe {
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_SAMPLE_BUFFERS, &mut samp_buf);
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_SAMPLES       , &mut samples );
            }
            let visualid = unsafe { (*visual_info).visualid };
            info!{
                "Matching FBConfig {}, visual ID {}, sample buffers = {}, samples = {}", 
                i, visualid, samp_buf, samples
            };
            if samp_buf > 0 && samples > best_sample_count {
                best_fbc = fbc;
                best_sample_count = samples;
            }
            unsafe { x::XFree(visual_info as *mut _); }
        }
        unsafe { 
            x::XFree(fbcs as *mut _); 
            let visual_info = glXGetVisualFromFBConfig(x_dpy, best_fbc);
            assert!(!visual_info.is_null());
            Ok(GLPixelFormat { dpy: self, visual_info, fbconfig: Some(best_fbc) })
        }
    }

    pub(super) fn create_window<'dpy>(&'dpy self, settings: &Settings) 
        -> Result<Window<'dpy>, super::Error>
    {
        let x_dpy = self.x_dpy;
        let parent = unsafe { x::XDefaultRootWindow(x_dpy) };
        
        let &Settings {
            mode, resizable, fully_opaque, ref opengl, allow_high_dpi
        } = settings;

        let (visual, depth, colormap) = match *opengl {
            Some(ref pixel_format) => {
                if self.glx.is_none() {
                    return Err(super::Error::Backend(Error::NoGLX));
                }
                let vi = unsafe { *pixel_format.0.visual_info };
                let colormap = unsafe {
                    x::XCreateColormap(x_dpy, parent, vi.visual, x::AllocNone)
                };
                (vi.visual, vi.depth, colormap)
            },
            None => {
                let depth = x::CopyFromParent;
                let visual = unsafe {
                    x::XDefaultVisual(x_dpy, self.screen_num)
                };
                let colormap = unsafe {
                    x::XCreateColormap(x_dpy, parent, visual, x::AllocNone)
                };
                (visual, depth, colormap)
            },
        };

        use super::window::Mode;
        let (w, h) = match mode {
            Mode::FixedSize(Extent2 { w, h }) => (w, h),
            Mode::FixedSizeFullScreen(Extent2 { w, h }) => unimplemented!{},
            Mode::DesktopSize => unimplemented!{},
            Mode::FullScreen => unimplemented!{},
        };
        let (x, y) = (0, 0);

        let border_thickness = 0;
        let class = x::InputOutput;
        let valuemask = x::CWBorderPixel | x::CWColormap | x::CWEventMask;
        let mut swa = x::XSetWindowAttributes {
            colormap,
            event_mask:
                x::ButtonReleaseMask      | x::EnterWindowMask | x::ButtonPressMask |
                x::LeaveWindowMask        | x::PointerMotionMask | 
                x::Button1MotionMask      |
                x::Button2MotionMask      | x::Button3MotionMask |
                x::Button4MotionMask      | x::Button5MotionMask |
                x::ButtonMotionMask       | x::KeymapStateMask |
                x::ExposureMask           | x::VisibilityChangeMask | 
                x::StructureNotifyMask    | /* ResizeRedirectMask | */
                x::SubstructureNotifyMask | x::SubstructureRedirectMask |
                x::FocusChangeMask        | x::PropertyChangeMask |
                x::ColormapChangeMask     | x::OwnerGrabButtonMask,
            background_pixmap    : 0,  
            background_pixel     : 0,  
            border_pixmap        : 0,  
            border_pixel         : 0,  
            bit_gravity          : 0,  
            win_gravity          : 0,  
            backing_store        : 0,  
            backing_planes       : 0,  
            backing_pixel        : 0,  
            save_under           : 0,  
            do_not_propagate_mask: 0,  
            override_redirect    : 0,  
            cursor               : 0,  
        };

        let x_window = unsafe { x::XCreateWindow(
            x_dpy, parent, x, y, w, h,
            border_thickness, depth, class as _, visual, valuemask, &mut swa
        )};

        if x_window == 0 {
            return Err(super::Error::CouldntCreateWindow);
        }

        unsafe {
            x::XFlush(x_dpy);
        }

        let wants_glx_window = {
            opengl.is_some() && self.glx.as_ref().unwrap().version >= Semver::new(1,3,0)
        };

        let glx_window = if wants_glx_window {
            let fbconfig = opengl.as_ref().unwrap().0.fbconfig.unwrap();
            Some(unsafe { glXCreateWindow(
                x_dpy, fbconfig, x_window, ptr::null_mut()
            )})
        } else { None };

        Ok(Window { dpy: self, x_window, colormap, glx_window })
    }


    pub(super) fn create_gl_context<'dpy>(&'dpy self, pf: &GLPixelFormat, cs: &GLContextSettings) 
        -> Result<GLContext<'dpy>, super::Error>
    {
        let x_dpy = self.x_dpy;

        if self.glx.is_none() {
            return Err(super::Error::Backend(Error::NoGLX));
        }

        let glx = self.glx.as_ref().unwrap();

        let &GLPixelFormat { visual_info, fbconfig, .. } = pf;

        unsafe {
            x::XSync(x_dpy, x::False);
            G_XLIB_ERROR_OCCURED.store(false, Ordering::SeqCst);
        }

        let (funcname, glx_context) = unsafe {
            if glx.version < Semver::new(1,3,0) {
                ("glXCreateContext", glXCreateContext(x_dpy, visual_info, ptr::null_mut(), x::True))
            } else if glx.version < Semver::new(1,4,0) 
                || (glx.version >= Semver::new(1,4,0) && !glx.ext.GLX_ARB_create_context) {
                ("glXCreateNewContext", glXCreateNewContext(
                    x_dpy, fbconfig.unwrap(), GLX_RGBA_TYPE, ptr::null_mut(), x::True
                ))
            } else {
                #[allow(non_snake_case)]
                let glXCreateContextAttribsARB = glx.ext.glXCreateContextAttribsARB.unwrap();
                let attribs_arb = glx.gen_arb_attribs(cs);
                ("glxCreateContextAttribsARB", glXCreateContextAttribsARB(
                    x_dpy, fbconfig.unwrap(), ptr::null_mut(), x::True, attribs_arb.as_ptr()
                ))
            }
        };

        unsafe {
            x::XSync(x_dpy, x::False);
            if glx_context.is_null() || G_XLIB_ERROR_OCCURED.load(Ordering::SeqCst) {
                return Err(super::Error::Backend(Error::FunctionName(funcname)));
            }

            info!("GLX context is direct: {}", glXIsDirect(x_dpy, glx_context));
            Ok(GLContext { dpy: self, glx_context })
        }
    }



    pub(super) fn create_software_gl_context<'dpy>(&'dpy self, pf: &GLPixelFormat, cs: &GLContextSettings) 
        -> Result<GLContext<'dpy>,super::Error>
    {
        unimplemented!()
    }
}


impl<'dpy> Window<'dpy> {

    pub(super) fn get_capabilities(&self) -> Capabilities {
        Capabilities {
            hide: WindowOpResult::Unimplemented,
            show: WindowOpResult::Success(()),
            set_title: WindowOpResult::Success(()),
            set_icon: WindowOpResult::Unimplemented,
            set_style: WindowOpResult::Unimplemented,
            recenter: WindowOpResult::Unimplemented,
            set_opacity: WindowOpResult::Unimplemented,
            maximize: WindowOpResult::Unimplemented,
            minimize: WindowOpResult::Unimplemented,
            restore: WindowOpResult::Unimplemented,
            raise: WindowOpResult::Unimplemented,
            enter_fullscreen: WindowOpResult::Unimplemented,
            leave_fullscreen: WindowOpResult::Unimplemented,
            set_minimum_size: WindowOpResult::Unimplemented,
            set_maximum_size: WindowOpResult::Unimplemented,
            move_absolute: WindowOpResult::Unimplemented,
            move_relative_to_parent: WindowOpResult::Unimplemented,
            move_relative_to_self: WindowOpResult::Unimplemented,
            resize: WindowOpResult::Unimplemented,
        }
    }

    unsafe extern fn _is_map_notify_callback(_x_dpy: *mut x::Display, ev: *mut x::XEvent, win: x::XPointer) -> i32 {
        let ev = ev.as_ref().unwrap();
        let xmap = x::XMapEvent::from(ev);
        let win = win as x::Window;
        (ev.get_type() == x::MapNotify && xmap.window == win) as i32
    }

    pub(super) fn show(&self) -> WindowOpResult<()> {
        unsafe {
            let x_dpy = self.dpy.x_dpy;
            let x_window = self.x_window;
            // if !self._is_mapped() {
                x::XMapRaised(x_dpy, x_window);
                /*
                 * This blocks
                let mut event: x::XEvent = mem::uninitialized();
                x::XIfEvent(x_dpy, &mut event,
                    Some(Self::is_map_notify_callback),
                    x_window as x::XPointer);
                */
                x::XSync(x_dpy, x::False); // Otherwise, it would be possible
                    // to swap buffer before the window is shown, which would
                    // have no effect.
            // }
            WindowOpResult::Success(())
        }
    }
    fn _is_mapped(&self) -> bool {
        unsafe {
            let mut attrs: x::XWindowAttributes = mem::uninitialized();
            x::XGetWindowAttributes(self.dpy.x_dpy, self.x_window, &mut attrs);
            attrs.map_state != x::IsUnmapped
        }
    }

    pub(super) fn create_child(&self, _settings: &Settings) -> Result<Self, super::Error> {
        // XCreateWindow() with parent set
        unimplemented!()
    }
    // XUnmapWindow()
    pub(super) fn hide(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    // XChangeProperty(), _NET_WM_ACTION_MINIMIZE and _NET_WM_ACTION_RESIZE as allowed actions 
    pub(super) fn maximize(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn minimize(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    // XRaiseWindow
    pub(super) fn raise(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    // Go from minimized to displayed
    pub(super) fn restore(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    // XChangeWindowAttributes() ??
    // XSetWMProperties() ??
    // http://wiki.tcl.tk/13409
    // _MOTIF_WM_HINTS (https://people.gnome.org/~tthurman/docs/metacity/xprops_8h-source.html)
    // _NET_WM_WINDOW_TYPE
    pub(super) fn set_style(&self, _style: &Style) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn enter_fullscreen(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn leave_fullscreen(&self) -> WindowOpResult<()> { WindowOpResult::Unimplemented }
    pub(super) fn set_icon(&self, _icon: Option<Icon>) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
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
    // XMoveWindow
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
    // XResizeWindow
    pub(super) fn resize(&self, _size: Extent2<u32>) -> WindowOpResult<()> {
        WindowOpResult::Unimplemented
    }
    pub(super) fn set_title(&self, title: &str) -> WindowOpResult<()> {
        unsafe {
            let mut title_prop: x::XTextProperty = mem::uninitialized();
            let title_ptr = CString::new(title).unwrap_or_default();
            let mut title_ptr = title_ptr.as_bytes_with_nul().as_ptr() as *mut u8;
            let title_ptr = &mut title_ptr as *mut _;
            let status = x::Xutf8TextListToTextProperty(
                self.dpy.x_dpy, mem::transmute(title_ptr), 1, x::XUTF8StringStyle, &mut title_prop
            );
            if status == x::Success as i32 {
                x::XSetTextProperty(self.dpy.x_dpy, self.x_window, &mut title_prop, self.dpy.atoms._NET_WM_NAME);
                x::XFree(title_prop.value as *mut _);
            }
            x::XFlush(self.dpy.x_dpy);
        }
        WindowOpResult::Success(())
    }
    pub(super) fn gl_swap_buffers(&self) {
        unsafe {
            glXSwapBuffers(self.dpy.x_dpy, match self.glx_window {
                Some(w) => w,
                None => self.x_window,
            });
        }
    }

    // XGetWindowAttributes
    pub(super) fn query_screenspace_size(&self) -> Extent2<u32> {
        unimplemented!()
    }
    // XGetWindowAttributes
    pub(super) fn query_canvas_size(&self) -> Extent2<u32> {
        unimplemented!()
    }
    // Easy
    pub(super) fn gl_set_swap_interval(&self, _interval: GLSwapInterval) -> Result<(),super::Error> { 
        unimplemented!()
    }
}



impl<'dpy> GLContext<'dpy> {
    pub(super) fn make_current(&self, win: &Window) {
        unsafe {
            match win.glx_window {
                Some(w) => glXMakeContextCurrent(
                    self.dpy.x_dpy, w, w, self.glx_context
                ),
                None => glXMakeCurrent(
                    self.dpy.x_dpy, win.x_window, self.glx_context
                ),
            };
        }
    }

    // NOTE: glXGetProcAddressARB doesn't need a bound context, unlike in WGL.
    pub(super) unsafe fn get_proc_address_raw(&self, name: *const c_char) -> Option<unsafe extern "C" fn()> {
        #[cfg(not(target_os = "linux"))]
        unimplemented!("We don't know how the situation is in OSes other than Linux!");
        #[cfg(target_os = "linux")]
        glXGetProcAddressARB(name as *const _)
    }
    pub(super) fn get_proc_address(&self, name: &str) -> Option<unsafe extern "C" fn()> {
        let name = CString::new(name).unwrap();
        unsafe {
            self.get_proc_address_raw(name.as_ptr())
        }
    }
}


