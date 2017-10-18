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
// Depending one the GLX version:
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

// Creating an RGBA pixmap:
// int width = 100;
// int height = 100;
// int depth = 32; // works fine with depth = 24
// int bitmap_pad = 32; // 32 for 24 and 32 bpp, 16, for 15&16
// int bytes_per_line = 0; // number of bytes in the client image between the start of one scanline and the start of the next
// Display *display=XOpenDisplay(0);
// unsigned char *image32=(unsigned char *)malloc(width*height*4);
// XImage *img = XCreateImage(display, CopyFromParent, depth, ZPixmap, 0, image32, width, height, bitmap_pad, bytes_per_line);
// Pixmap p = XCreatePixmap(display, XDefaultRootWindow(display), width, height, depth);
// XGCValues gcvalues;
// GC gc = XCreateGC(display, p, 0, &gcvalues);
// XPutImage(display, p, gc, img, 0, 0, 0, 0, width, height); // 0, 0, 0, 0 are src x,y and dst x,y


extern crate x11;
use self::x11::xlib as x;
use self::x11::glx::*;
use self::x11::glx::arb::*;
use std::fmt::{self, Formatter};
use std::ptr;
use std::mem;
use std::ffi::*;
use std::os::raw::{c_char, c_uchar, c_int, c_long, c_ulong};
use std::sync::atomic::{ATOMIC_BOOL_INIT, AtomicBool, Ordering};
use std::slice;
use libc;

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

// TODO: move to `vek`.
#[derive(Debug, Copy, Clone, Hash, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Display {
    pub x_dpy: *mut x::Display,
    pub atoms: PreparedAtoms,
    pub screen: *mut x::Screen, // NOTE: Nothing says it needs to be freed, so we don't.
    pub screen_num: c_int,
    pub root: x::Window,
    pub glx: Option<Glx>,
    pub usable_viewport: Rect<u32>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Glx {
    pub version: Semver,
    pub get_proc_address: types::glXGetProcAddress,
    pub ext: GlxExt,
    pub error_base: c_int,
    pub event_base: c_int,
}

#[derive(Debug, PartialEq)]
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
    MissingExtensionToGLX,
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
        match *self {
            Error::NoXDisplayForName { ref name } => {
                let name = unsafe { CString::from_raw(x::XDisplayName(
                    match name {
                        &None => ptr::null(),
                        &Some(ref name) => &name as *const _ as *const i8,
                    }
                ))};
                let name = name.to_str().unwrap_or("<utf-8 conversion error>");
                write!(fmt, "\"{}\" is not a valid X display", name)
            },
            Error::NoGLX => {
                write!(fmt, "The GLX extension is not present")
            },
            Error::UnsupportedGLContextSettings => {
                write!(fmt, "Unsupported OpenGL context settings")
            },
            Error::MissingExtensionToGLX => {
                write!(fmt, "Functionality requires some extension to GLX, but it is not present.")
            },
            Error::FunctionName(name) => {
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
        #[allow(non_snake_case)]
        impl PreparedAtoms {
            fn fetch(x_dpy: *mut x::Display) -> Self {
                $(
                    let $atom = CString::new(stringify!($atom)).unwrap();
                    let $atom = unsafe { x::XInternAtom(
                        x_dpy, $atom.as_ptr(), x::False // Don't create
                    )};
                    match $atom {
                        0 => warn!("Atom not present: {}", stringify!($atom)),
                        _ => info!("Found atom {} = {}", stringify!($atom), $atom),
                    };
                )+
                Self { $($atom,)+ }
            }
        }
    }
}

atoms!(

    // Some base atoms
    UTF8_STRING
    PRIMARY
    SECONDARY
    CLIPBOARD

    // One mindlessly grabbed from SDL2
    XKLAVIER_STATE

    // Some ICCCM atoms
    WM_PROTOCOLS
    WM_DELETE_WINDOW
    WM_TAKE_FOCUS

    // EWMH atoms
    _NET_SUPPORTED
    _NET_CLIENT_LIST
    _NET_NUMBER_OF_DESKTOPS
    _NET_DESKTOP_GEOMETRY
    _NET_DESKTOP_VIEWPORT
    _NET_CURRENT_DESKTOP
    _NET_DESKTOP_NAMES
    _NET_ACTIVE_WINDOW
    _NET_WORKAREA
    _NET_SUPPORTING_WM_CHECK
    _NET_VIRTUAL_ROOTS
    _NET_DESKTOP_LAYOUT
    _NET_SHOWING_DESKTOP

    _NET_CLOSE_WINDOW
    _NET_MOVERESIZE_WINDOW
    _NET_WM_MOVERESIZE
    _NET_RESTACK_WINDOW
    _NET_REQUEST_FRAME_EXTENTS

    _NET_WM_NAME
    _NET_WM_VISIBLE_NAME
    _NET_WM_ICON_NAME
    _NET_WM_VISIBLE_ICON_NAME
    _NET_WM_DESKTOP

    _NET_WM_WINDOW_TYPE
    _NET_WM_WINDOW_TYPE_DESKTOP
    _NET_WM_WINDOW_TYPE_DOCK
    _NET_WM_WINDOW_TYPE_TOOLBAR
    _NET_WM_WINDOW_TYPE_MENU
    _NET_WM_WINDOW_TYPE_UTILITY
    _NET_WM_WINDOW_TYPE_SPLASH
    _NET_WM_WINDOW_TYPE_DIALOG
    _NET_WM_WINDOW_TYPE_DROPDOWN_MENU
    _NET_WM_WINDOW_TYPE_POPUP_MENU
    _NET_WM_WINDOW_TYPE_TOOLTIP
    _NET_WM_WINDOW_TYPE_NOTIFICATION
    _NET_WM_WINDOW_TYPE_COMBO
    _NET_WM_WINDOW_TYPE_DND
    _NET_WM_WINDOW_TYPE_NORMAL

    _NET_WM_STATE
    _NET_WM_STATE_MODAL
    _NET_WM_STATE_STICKY
    _NET_WM_STATE_MAXIMIZED_VERT
    _NET_WM_STATE_MAXIMIZED_HORZ
    _NET_WM_STATE_SHADED
    _NET_WM_STATE_SKIP_TASKBAR
    _NET_WM_STATE_SKIP_PAGER
    _NET_WM_STATE_HIDDEN
    _NET_WM_STATE_FULLSCREEN
    _NET_WM_STATE_ABOVE
    _NET_WM_STATE_BELOW
    _NET_WM_STATE_DEMANDS_ATTENTION
    _NET_WM_STATE_FOCUSED

    _NET_WM_ALLOWED_ACTIONS
    _NET_WM_ACTION_MOVE
    _NET_WM_ACTION_RESIZE
    _NET_WM_ACTION_MINIMIZE
    _NET_WM_ACTION_SHADE
    _NET_WM_ACTION_STICK
    _NET_WM_ACTION_MAXIMIZE_HORZ
    _NET_WM_ACTION_MAXIMIZE_VERT
    _NET_WM_ACTION_FULLSCREEN
    _NET_WM_ACTION_CHANGE_DESKTOP
    _NET_WM_ACTION_CLOSE
    _NET_WM_ACTION_ABOVE
    _NET_WM_ACTION_BELOW

    _NET_WM_STRUT
    _NET_WM_STRUT_PARTIAL
    _NET_WM_ICON_GEOMETRY
    // This is an array of 32bit packed CARDINAL ARGB with high byte being A, low byte being B. The first two cardinals are width, height. Data is in rows, left to right and top to bottom.
    _NET_WM_ICON

    _NET_WM_PID
    _NET_WM_HANDLED_ICONS
    _NET_WM_USER_TIME
    _NET_WM_USER_TIME_WINDOW
    _NET_FRAME_EXTENTS
    _NET_WM_OPAQUE_REGION
    _NET_WM_BYPASS_COMPOSITOR

    _NET_WM_PING
    _NET_WM_SYNC_REQUEST
    _NET_WM_FULLSCREEN_MONITORS
    _NET_WM_FULL_PLACEMENT
    _NET_WM_WINDOW_OPACITY // Doesn't seem to be defined officially ??

    // X Drag'n Drop atoms
    // Also don't forget to check:
    // https://www.freedesktop.org/wiki/Draganddropwarts/
    XdndAware
    XdndEnter
    XdndPosition
    XdndLeave
    XdndStatus
    XdndTypeList
    XdndDrop
    XdndFinished
    XdndSelection
    XdndActionCopy
    XdndActionMove
    XdndActionLink
    XdndActionAsk
    XdndActionPrivate
    XdndActionList
    XdndActionDescription
    XdndProxy
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
                        None => {
                            warn!("Couldn't load `{}`", stringify!($func));
                            None
                        },
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
    glXSwapIntervalEXT
    glXSwapIntervalMESA
    glXGetSwapIntervalMESA
    glXSwapIntervalSGI
    glXCreateContextAttribsARB
));


// TODO: Send a PR to x11-rs.
mod xx {
    pub const GLX_CONTEXT_ES_PROFILE_BIT_EXT             : i32 = 0x00000004;
    // pub const GLX_CONTEXT_ES2_PROFILE_BIT_EXT            : i32 = 0x00000004;
    pub const GLX_CONTEXT_ROBUST_ACCESS_BIT_ARB          : i32 = 0x00000004;
    pub const GLX_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB: i32 = 0x8256;
    // pub const GLX_NO_RESET_NOTIFICATION_ARB              : i32 = 0x8261;
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
            0, // GLX_SAMPLE_BUFFERS_ARB attrib, see below
            0, // GLX_SAMPLE_BUFFERS_ARB value, see below
            0, // GLX_SAMPLES_ARB attrib, see below
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
    fn gen_fbconfig_attribs(&self, settings: &GLPixelFormatSettings) -> [c_int; 43] {
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
            GLX_CONFIG_CAVEAT, GLX_DONT_CARE, // NOTE: Setting it to GLX_NONE is very strict.
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

    // WISH: Grab from _XPrintDefaultError in Xlib's sources
    unsafe extern fn _x_generic_error_handler(_dpy: *mut x::Display, e: *mut x::XErrorEvent) -> c_int {
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
            //
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
                    Error::NoXDisplayForName { 
                        name: x_display_name.map(|s| CString::new(
                            s.to_bytes_with_nul().to_owned()
                        ).unwrap_or_default())
                    }
                ));
            }

            let protocol_version  = x::XProtocolVersion(x_dpy);
            let protocol_revision = x::XProtocolRevision(x_dpy);
            let screen_count      = x::XScreenCount(x_dpy);
            let vendor_release    = x::XVendorRelease(x_dpy);
            let display_string    = CStr::from_ptr(x::XDisplayString(x_dpy)).to_string_lossy();
            let server_vendor     = CStr::from_ptr(x::XServerVendor(x_dpy) ).to_string_lossy();
            info!("Opened X11 display `{}`", display_string);
            info!("X Protocol version {}, revision {}", protocol_version, protocol_revision);
            info!("Vendor: `{}`, release {}", server_vendor, vendor_release);
            info!("Screen count: {}", screen_count);

            let screen = x::XDefaultScreenOfDisplay(x_dpy);
            let screen_num = x::XDefaultScreen(x_dpy);
            let root = x::XRootWindowOfScreen(screen);
            let atoms = PreparedAtoms::fetch(x_dpy);
            let glx = Self::query_glx(x_dpy, screen_num);
            let usable_viewport = Self::query_usable_viewport(x_dpy, screen_num, &atoms);

            Ok(Self { x_dpy, atoms, screen, screen_num, root, glx, usable_viewport })
        }
    }

    // FIXME: If we can't, return the whole screen size.
    fn query_usable_viewport(x_dpy: *mut x::Display, screen_num: c_int, atoms: &PreparedAtoms) -> Rect<u32> {
        let mut real_type: x::Atom = 0;
        let mut real_format: c_int = 0;
        let mut items_read: c_ulong = 0;
        let mut items_left: c_ulong = 0;
        let mut propdata: *mut c_uchar = ptr::null_mut();

        // XXX: Dubious
        let fallback = unsafe { Rect {
            x: 0, y: 0,
            w: x::XDisplayWidth(x_dpy, screen_num) as u32,
            h: x::XDisplayHeight(x_dpy, screen_num) as u32,
        }};

        let status = unsafe { x::XGetWindowProperty(
            x_dpy, x::XDefaultRootWindow(x_dpy), atoms._NET_WORKAREA,
            0, 4, x::False, x::XA_CARDINAL,
            &mut real_type, &mut real_format, &mut items_read,
            &mut items_left, &mut propdata
        )};
        let usable = if status == x::Success as _ && items_read >= 4 {
            let p = unsafe {
                slice::from_raw_parts(propdata as *const c_long, items_read as _)
            };
            let usable = Rect {
                x: p[0] as u32, y: p[1] as u32, w: p[2] as u32, h: p[3] as u32
            };
            usable
        } else {
            fallback
        };

        unsafe {
            x::XFree(propdata as _);
        }
        usable
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
        unimplemented!("We don't know how the situation is in OSes other than Linux! This could require moving to x11-dl.");
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
        //
        // glXChooseFBConfig's man page describes the sorting order, which
        // in general favors more lightweight configs: what matters most to us
        // is that the sorting order favors single-buffered configs, and is
        // apparently oblivious to MSAA parameters.
        //
        // So what we've got to do is run through the list of candidates and
        // stop at the first that supports double buffering and exactly our
        // MSAA params. If there's none, we'll just select the first one.

        let mut best_fbc = unsafe { *fbcs };
        let mut best_fbc_i = 0;
        let mut is_fbconfig_chosen = false;

        for i in 0..fbcount {
            let fbc = unsafe { *fbcs.offset(i as isize) };
            let visual_info = unsafe {
                glXGetVisualFromFBConfig(x_dpy, fbc)
            };
            if visual_info.is_null() {
                continue;
            }
            let mut sample_buffers          : c_int = 0;
            let mut samples                 : c_int = 0;
            let mut fbconfig_id             : c_int = 0; 
            let mut buffer_size             : c_int = 0; 
            let mut level                   : c_int = 0; 
            let mut stereo                  : c_int = 0; 
            let mut doublebuffer            : c_int = 0;
            let mut aux_buffers             : c_int = 0; 
            let mut red_size                : c_int = 0; 
            let mut green_size              : c_int = 0; 
            let mut blue_size               : c_int = 0; 
            let mut alpha_size              : c_int = 0; 
            let mut depth_size              : c_int = 0; 
            let mut stencil_size            : c_int = 0; 
            let mut accum_red_size          : c_int = 0; 
            let mut accum_green_size        : c_int = 0; 
            let mut accum_blue_size         : c_int = 0; 
            let mut accum_alpha_size        : c_int = 0; 
            let mut render_type             : c_int = 0; 
            let mut drawable_type           : c_int = 0; 
            let mut x_renderable            : c_int = 0; 
            let mut visual_id               : c_int = 0; 
            let mut x_visual_type           : c_int = 0; 
            let mut config_caveat           : c_int = 0; 
            let mut transparent_type        : c_int = 0; 
            let mut transparent_index_value : c_int = 0; 
            let mut transparent_red_value   : c_int = 0; 
            let mut transparent_green_value : c_int = 0; 
            let mut transparent_blue_value  : c_int = 0; 
            let mut transparent_alpha_value : c_int = 0; 
            let mut max_pbuffer_width       : c_int = 0; 
            let mut max_pbuffer_height      : c_int = 0; 
            let mut max_pbuffer_pixels      : c_int = 0; 
            unsafe {
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_SAMPLE_BUFFERS         , &mut sample_buffers         );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_SAMPLES                , &mut samples                );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_FBCONFIG_ID            , &mut fbconfig_id            );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_BUFFER_SIZE            , &mut buffer_size            );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_LEVEL                  , &mut level                  );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_DOUBLEBUFFER           , &mut stereo                 );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_STEREO                 , &mut doublebuffer           );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_AUX_BUFFERS            , &mut aux_buffers            );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_RED_SIZE               , &mut red_size               );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_GREEN_SIZE             , &mut green_size             );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_BLUE_SIZE              , &mut blue_size              );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_ALPHA_SIZE             , &mut alpha_size             );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_DEPTH_SIZE             , &mut depth_size             );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_STENCIL_SIZE           , &mut stencil_size           );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_ACCUM_RED_SIZE         , &mut accum_red_size         );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_ACCUM_GREEN_SIZE       , &mut accum_green_size       );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_ACCUM_BLUE_SIZE        , &mut accum_blue_size        );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_ACCUM_ALPHA_SIZE       , &mut accum_alpha_size       );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_RENDER_TYPE            , &mut render_type            );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_DRAWABLE_TYPE          , &mut drawable_type          );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_X_RENDERABLE           , &mut x_renderable           );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_VISUAL_ID              , &mut visual_id              );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_X_VISUAL_TYPE          , &mut x_visual_type          );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_CONFIG_CAVEAT          , &mut config_caveat          );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_TRANSPARENT_TYPE       , &mut transparent_type       );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_TRANSPARENT_INDEX_VALUE, &mut transparent_index_value);
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_TRANSPARENT_RED_VALUE  , &mut transparent_red_value  );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_TRANSPARENT_GREEN_VALUE, &mut transparent_green_value);
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_TRANSPARENT_BLUE_VALUE , &mut transparent_blue_value );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_TRANSPARENT_ALPHA_VALUE, &mut transparent_alpha_value);
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_MAX_PBUFFER_WIDTH      , &mut max_pbuffer_width      );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_MAX_PBUFFER_HEIGHT     , &mut max_pbuffer_height     );
                glXGetFBConfigAttrib(x_dpy, fbc, GLX_MAX_PBUFFER_PIXELS     , &mut max_pbuffer_pixels     );
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
                best_fbc = fbc;
                best_fbc_i = i;
                // Don't `break`, ensure we run through the whole list first
                // so we can log them all.
            }
        }
        info!("Chosen FBConfig n°{}", best_fbc_i);
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

        let _ = allow_high_dpi;
        let _ = fully_opaque;

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
        let (w, h, maximized, fullscreen) = match mode {
            Mode::FixedSize(Extent2 { w, h }) => (w, h, false, false),
            // FIXME: Don't give `1` as extents. The accuracy is relied upon later.
            Mode::Maximized => (1, 1, true, false),
            Mode::FullScreen => (1, 1, false, true),
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
            let mut protocols = [ 
                self.atoms.WM_DELETE_WINDOW,
                self.atoms._NET_WM_PING,
                self.atoms.WM_TAKE_FOCUS,
            ];
            x::XSetWMProtocols(
                x_dpy, x_window, protocols.as_mut_ptr(), protocols.len() as _
            );

            let pid = libc::getpid();
            if pid > 0 {
                x::XChangeProperty(
                    x_dpy, x_window, self.atoms._NET_WM_PID, 
                    x::XA_CARDINAL, 32, x::PropModeReplace,
                    &pid as *const _ as *const _, 
                    1
                );
            }
            /*
            x::XChangeProperty(
                x_dpy, x_window, self.atoms.XdndAware, 
                x::XA_ATOM, 32, x::PropModeReplace,
                &xdnd_version as *const _ as *const _, 
                1
            );
            */
        }

        // TODO: Move this to set_minimum_size() and friends
        let sizehints = x::XSizeHints {
            flags: x::PPosition | x::PSize | x::PMinSize | x::PMaxSize
                 /*| x::PResizeInc | x::PAspect*/,
            x, y, 
            width: w as _, 
            height: h as _,
            min_width:  if resizable { 0 } else { w } as _, 
            min_height: if resizable { 0 } else { h } as _,
            max_width:  if resizable { 999999 } else { w } as _, 
            max_height: if resizable { 999999 } else { h } as _,
            width_inc: 1,
            height_inc: 1,
            min_aspect: x::AspectRatio { x: 0, y: 0 },
            max_aspect: x::AspectRatio { x: 0, y: 0 },
            base_width: 0, 
            base_height: 0,
            win_gravity: 0,
        };
        // TODO: leverage the UrgencyHint for messageboxes and stuff
        let wmhints = x::XWMHints {
            flags: x::InputHint /*| x::WindowGroupHint | x::IconPixmapHint | ...*/,
            input: x::True,
            .. unsafe { mem::zeroed() }
            /*
            initial_state,
            icon_pixmap, icon_window, icon_x, icon_y, icon_mask,
            window_group: window_group,
            */
        };
        // TODO: readlink() on:
        // - /proc/<pid>/exe on Linux, 
        // - /proc/<pid>/file on FreeBSD.
        let classname = b"dmc_app\0";
        let classhint = x::XClassHint {
            res_name: classname as *const _ as *mut _,
            res_class: classname as *const _ as *mut _,
        };

        unsafe {
            // We must do this because the structs might be extended in the
            // future and only the XAlloc* functions know how big they are.
            // Silly if you ask me.
            let sizehints_buf = x::XAllocSizeHints();
            let classhint_buf = x::XAllocClassHint();
            let wmhints_buf = x::XAllocWMHints();

            *sizehints_buf = sizehints;
            *classhint_buf = classhint;
            *wmhints_buf = wmhints;

            let argc = 0;
            let argv = ptr::null_mut();
            let window_name = ptr::null_mut();
            let icon_name = ptr::null_mut();
            // replaces x::XSetWMNormalHints(x_dpy, x_window, &mut hints);
            x::XSetWMProperties(
                x_dpy, x_window, window_name, icon_name, argv, argc,
                sizehints_buf, wmhints_buf, classhint_buf
            );
            x::XFree(sizehints_buf as _);
            x::XFree(classhint_buf as _);
            x::XFree(wmhints_buf as _);

            let always_on_top = false;
            let skip_taskbar = false;
            let input_focus = false;
            let mut atoms: [x::Atom; 16] = [0; 16];
            let mut count = 0;
            if always_on_top {
                atoms[count] = self.atoms._NET_WM_STATE_ABOVE;
                count += 1;
            }
            if skip_taskbar {
                atoms[count] = self.atoms._NET_WM_STATE_SKIP_TASKBAR;
                count += 1;
                atoms[count] = self.atoms._NET_WM_STATE_SKIP_PAGER;
                count += 1;
            }
            if input_focus {
                atoms[count] = self.atoms._NET_WM_STATE_FOCUSED;
                count += 1;
            }
            if maximized {
                atoms[count] = self.atoms._NET_WM_STATE_MAXIMIZED_VERT;
                count += 1;
                atoms[count] = self.atoms._NET_WM_STATE_MAXIMIZED_HORZ;
                count += 1;
            }
            if fullscreen {
                atoms[count] = self.atoms._NET_WM_STATE_FULLSCREEN;
                count += 1;
            }
            if count > 0 {
                x::XChangeProperty(
                    x_dpy, x_window, self.atoms._NET_WM_STATE, x::XA_ATOM, 32,
                    x::PropModeReplace, atoms.as_mut_ptr() as *mut _, count as _
                );
            } else {
                x::XDeleteProperty(x_dpy, x_window, self.atoms._NET_WM_STATE);
            }

            // TODO: There are many other possible types of window.
            let mut wintype = self.atoms._NET_WM_WINDOW_TYPE_NORMAL;
            x::XChangeProperty(
                x_dpy, x_window, self.atoms._NET_WM_WINDOW_TYPE, x::XA_ATOM, 32,
                x::PropModeReplace, &mut wintype as *mut _ as *mut _, 1
            );
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

        Ok(Window { dpy: self, x_window, colormap, glx_window, })
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
                   || (glx.version >= Semver::new(1,4,0) && !glx.ext.GLX_ARB_create_context)
            {
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


    pub(super) fn create_software_gl_context<'dpy>(&'dpy self, _pf: &GLPixelFormat, _cs: &GLContextSettings) 
        -> Result<GLContext<'dpy>,super::Error>
    {
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
        unimplemented!("We don't know how the situation is in OSes other than Linux! This could require moving to x11-dl.");
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


impl<'dpy> Window<'dpy> {

    pub(super) fn gl_swap_buffers(&self) {
        unsafe {
            glXSwapBuffers(self.dpy.x_dpy, match self.glx_window {
                Some(w) => w,
                None => self.x_window,
            });
        }
    }

    pub(super) fn gl_set_swap_interval(&self, interval: GLSwapInterval) -> Result<(),super::Error> { 

        let glx = self.dpy.glx.as_ref().unwrap();

        let interval: c_int = match interval {
            GLSwapInterval::LimitFps(_) => unreachable!{} /* Implemented globally in mod.rs instead */,
            GLSwapInterval::VSync => 1,
            GLSwapInterval::Immediate => 0,
            GLSwapInterval::LateSwapTearing => {
                if !glx.ext.GLX_EXT_swap_control_tear {
                    return Err(super::Error::Backend(Error::MissingExtensionToGLX));
                }
                -1
            },
            GLSwapInterval::Interval(i) => {
                if i < 0 && !glx.ext.GLX_EXT_swap_control_tear {
                    return Err(super::Error::Backend(Error::MissingExtensionToGLX));
                }
                i
            },
        };

        if glx.ext.GLX_EXT_swap_control && self.glx_window.is_some() {
            let ssi = glx.ext.glXSwapIntervalEXT.unwrap();
            unsafe {
                ssi(self.dpy.x_dpy, self.glx_window.unwrap(), interval);
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
            warn!("There's no extension that could set the swap interval!");
            Err(super::Error::Backend(Error::MissingExtensionToGLX))
        }
    }

    pub(super) fn get_capabilities(&self) -> Capabilities {
        Capabilities {
            hide: WindowOpResult::Unimplemented,
            show: WindowOpResult::Success(()),
            set_title: WindowOpResult::Success(()),
            clear_icon: WindowOpResult::Unimplemented,
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
    pub(super) fn clear_icon(&self) -> WindowOpResult<()> {
        let x_dpy = self.dpy.x_dpy;
        let x_window = self.x_window;
        #[allow(non_snake_case)]
        let _NET_WM_ICON = self.dpy.atoms._NET_WM_ICON;

        unsafe {
            x::XDeleteProperty(x_dpy, x_window, _NET_WM_ICON);
        }
        WindowOpResult::Success(())
    }
    pub(super) fn set_icon(&self, icon: Icon) -> WindowOpResult<()> {
        let x_dpy = self.dpy.x_dpy;
        let x_window = self.x_window;
        #[allow(non_snake_case)]
        let _NET_WM_ICON = self.dpy.atoms._NET_WM_ICON;

        let (w, h) = (icon.size.w, icon.size.h);
        let mut prop = Vec::<u32>::with_capacity((2 + w * h) as _);
        prop.push(w);
        prop.push(h);
        for y in 0..h {
            for x in 0..w {
                let p: Rgba<u8> = icon[(x, y)];
                let argb: u32 = 
                      (p.a as u32) << 24
                    | (p.r as u32) << 16
                    | (p.g as u32) << 8
                    | (p.b as u32);
                prop.push(argb);
            }
        }
        unsafe { 
            x::XChangeProperty(
                x_dpy, x_window, _NET_WM_ICON, x::XA_CARDINAL, 32, 
                x::PropModeReplace, prop.as_ptr() as _, prop.len() as _
            );
            x::XFlush(x_dpy);
        }

        WindowOpResult::Success(())
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

    // XGetWindowAttributes
    pub(super) fn query_screenspace_size(&self) -> Extent2<u32> {
        unimplemented!()
    }
    // XGetWindowAttributes
    pub(super) fn query_canvas_size(&self) -> Extent2<u32> {
        unimplemented!()
    }
}

