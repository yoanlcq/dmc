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
// Context *display=XOpenDisplay(0);
// unsigned char *image32=(unsigned char *)malloc(width*height*4);
// XImage *img = XCreateImage(display, CopyFromParent, depth, ZPixmap, 0, image32, width, height, bitmap_pad, bytes_per_line);
// Pixmap p = XCreatePixmap(display, XDefaultRootWindow(display), width, height, depth);
// XGCValues gcvalues;
// GC gc = XCreateGC(display, p, 0, &gcvalues);
// XPutImage(display, p, gc, img, 0, 0, 0, 0, width, height); // 0, 0, 0, 0 are src x,y and dst x,y


extern crate x11;
extern crate libc;
extern crate libudev_sys as udev;
extern crate libevdev_sys;

use std::fmt::{self, Debug, Formatter};
use std::ptr;
use std::mem;
use std::ffi::*;
use std::os::raw::{c_char, c_uchar, c_int, c_uint, c_long, c_ulong, c_void};
use std::sync::atomic::{ATOMIC_BOOL_INIT, AtomicBool, Ordering};
use std::slice;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::time::{Instant, Duration};
use std::collections::HashMap;

use self::x11::xlib as x;
use self::x11::xinput as xi;
use self::x11::xinput2 as xi2;
use self::x11::xrender;
use self::x11::keysym::*;
use self::x11::glx::*;
use self::x11::glx::arb::*;

use self::libevdev_sys::evdev;

use gl::*;
use window::*;
use cursor::*;
use event::*;
use hid::*;
use timeout::Timeout;
use super::{Extent2, Vec2, Rgba, Rect};
use decision::Decision;
use semver::Semver;


static mut G_XLIB_ERROR_OCCURED: AtomicBool = ATOMIC_BOOL_INIT;
static mut G_GLX_ERROR_BASE: i32 = 0;
static mut G_GLX_EVENT_BASE: i32 = 0;
static mut G_XRENDER_ERROR_BASE: i32 = 0;
static mut G_XRENDER_EVENT_BASE: i32 = 0;
static mut G_XI_ERROR_BASE: i32 = 0;
static mut G_XI_EVENT_BASE: i32 = 0;
static mut G_XI_OPCODE: i32 = 0;

// WISH: Grab from _XPrintDefaultError in Xlib's sources
unsafe extern fn xlib_generic_error_handler(_shared_context: *mut x::Display, e: *mut x::XErrorEvent) -> c_int {
    // NOTE: DO NOT make requests to the X server within X error handlers such as this one.
    G_XLIB_ERROR_OCCURED.store(true, Ordering::SeqCst);
    let e = *e;
    error!("Received X error: XErrorEvent {{ type: {}, display: {:?}, resourceid: {}, serial: {}, error_code: {}, request_code: {}, minor_code: {} }}", e.type_, e.display, e.resourceid, e.serial, e.error_code, e.request_code, e.minor_code);
    0
}



pub mod types {
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

// TODO: Send a PR to x11-rs.
// Missing items for X11
pub mod xx {
    pub const GLX_CONTEXT_ES_PROFILE_BIT_EXT             : i32 = 0x00000004;
    pub const GLX_CONTEXT_ES2_PROFILE_BIT_EXT            : i32 = 0x00000004;
    pub const GLX_CONTEXT_ROBUST_ACCESS_BIT_ARB          : i32 = 0x00000004;
    pub const GLX_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB: i32 = 0x8256;
    pub const GLX_NO_RESET_NOTIFICATION_ARB              : i32 = 0x8261;
    pub const GLX_LOSE_CONTEXT_ON_RESET_ARB              : i32 = 0x8252;
}

// TODO: Send a PR to x11-rs.
// Missing items for XInput
pub mod xxi {
    pub const NoSuchExtension: i32 = 1;
}

pub mod xxrender {
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    #[repr(u32)]
    pub enum PictStandard {
        ARGB32 = 0,
        RGB24  = 1,
        A8	   = 2,
        A4	   = 3,
        A1	   = 4,
        NUM	   = 5,
    }
}


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum NetWMStateAction {
    Remove = 0,
    Add    = 1,
    Toggle = 2,
}
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(u32)]
pub enum BypassCompositor {
    NoPreference = 0,
    Yes = 1,
    No = 2,
}
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum NetWMWindowType {
    Desktop,
    Dock,
    Toolbar,
    Menu,
    Utility,
    Splash,
    Dialog,
    DropdownMenu,
    PopupMenu,
    Tooltip,
    Notification,
    Combo,
    DND,
    Normal,
}

pub mod mwm {
    use super::{c_long, c_ulong};
    pub const HINTS_FUNCTIONS   : c_ulong = 1 << 0;
    pub const HINTS_DECORATIONS : c_ulong = 1 << 1;
    pub const DECOR_ALL         : c_ulong = 1 << 0;
    pub const DECOR_BORDER      : c_ulong = 1 << 1;
    pub const DECOR_RESIZEH     : c_ulong = 1 << 2;
    pub const DECOR_TITLE       : c_ulong = 1 << 3;
    pub const DECOR_MENU        : c_ulong = 1 << 4;
    pub const DECOR_MINIMIZE    : c_ulong = 1 << 5;
    pub const DECOR_MAXIMIZE    : c_ulong = 1 << 6;
    pub const FUNC_ALL          : c_ulong = 1 << 0;
    pub const FUNC_RESIZE       : c_ulong = 1 << 1;
    pub const FUNC_MOVE         : c_ulong = 1 << 2;
    pub const FUNC_MINIMIZE     : c_ulong = 1 << 3;
    pub const FUNC_MAXIMIZE     : c_ulong = 1 << 4;
    pub const FUNC_CLOSE        : c_ulong = 1 << 5;

    #[repr(C)]
    pub struct WMHints {
        pub flags      : c_ulong,
        pub functions  : c_ulong,
        pub decorations: c_ulong,
        pub input_mode : c_long,
        pub state      : c_ulong,
    }
}

macro_rules! xc_glyphs {
    ($($name:ident $val:tt)+) => {
        $(pub const $name: u32 = $val;)+
    };
}
xc_glyphs!{
    XC_num_glyphs 154
    XC_X_cursor 0
    XC_arrow 2
    XC_based_arrow_down 4
    XC_based_arrow_up 6
    XC_boat 8
    XC_bogosity 10
    XC_bottom_left_corner 12
    XC_bottom_right_corner 14
    XC_bottom_side 16
    XC_bottom_tee 18
    XC_box_spiral 20
    XC_center_ptr 22
    XC_circle 24
    XC_clock 26
    XC_coffee_mug 28
    XC_cross 30
    XC_cross_reverse 32
    XC_crosshair 34
    XC_diamond_cross 36
    XC_dot 38
    XC_dotbox 40
    XC_double_arrow 42
    XC_draft_large 44
    XC_draft_small 46
    XC_draped_box 48
    XC_exchange 50
    XC_fleur 52
    XC_gobbler 54
    XC_gumby 56
    XC_hand1 58
    XC_hand2 60
    XC_heart 62
    XC_icon 64
    XC_iron_cross 66
    XC_left_ptr 68
    XC_left_side 70
    XC_left_tee 72
    XC_leftbutton 74
    XC_ll_angle 76
    XC_lr_angle 78
    XC_man 80
    XC_middlebutton 82
    XC_mouse 84
    XC_pencil 86
    XC_pirate 88
    XC_plus 90
    XC_question_arrow 92
    XC_right_ptr 94
    XC_right_side 96
    XC_right_tee 98
    XC_rightbutton 100
    XC_rtl_logo 102
    XC_sailboat 104
    XC_sb_down_arrow 106
    XC_sb_h_double_arrow 108
    XC_sb_left_arrow 110
    XC_sb_right_arrow 112
    XC_sb_up_arrow 114
    XC_sb_v_double_arrow 116
    XC_shuttle 118
    XC_sizing 120
    XC_spider 122
    XC_spraycan 124
    XC_star 126
    XC_target 128
    XC_tcross 130
    XC_top_left_arrow 132
    XC_top_left_corner 134
    XC_top_right_corner 136
    XC_top_side 138
    XC_top_tee 140
    XC_trek 142
    XC_ul_angle 144
    XC_umbrella 146
    XC_ur_angle 148
    XC_watch 150
    XC_xterm 152
}

macro_rules! keys_to_x_keysyms {
    ($($Key:ident $x_keysym:ident,)+) => {
        fn key_to_x_keysym(key: Key) -> Option<x::KeySym> {
            match key {
                $(Key::$Key => Some($x_keysym as _),)+
                Key::Other(x) => Some(x as _),
                _ => None,
            }
        }
        fn x_keysym_to_key(x_keysym: x::KeySym) -> Key {
            match x_keysym as _ {
                $($x_keysym => Key::$Key,)+
                x @ _ => Key::Other(x as _),
            }
        }
    };
}

keys_to_x_keysyms!{
    Num1             XK_1            ,
    Num2             XK_2            ,
    Num3             XK_3            ,
    Num4             XK_4            ,
    Num5             XK_5            ,
    Num6             XK_6            ,
    Num7             XK_7            ,
    Num8             XK_8            ,
    Num9             XK_9            ,
    Num0             XK_0            ,
    A                XK_a               ,
    B                XK_b               ,
    C                XK_c               ,
    D                XK_d               ,
    E                XK_e               ,
    F                XK_f               ,
    G                XK_g               ,
    H                XK_h               ,
    I                XK_i               ,
    J                XK_j               ,
    K                XK_k               ,
    L                XK_l               ,
    M                XK_m               ,
    N                XK_n               ,
    O                XK_o               ,
    P                XK_p               ,
    Q                XK_q               ,
    R                XK_r               ,
    S                XK_s               ,
    T                XK_t               ,
    U                XK_u               ,
    V                XK_v               ,
    W                XK_w               ,
    X                XK_x               ,
    Y                XK_y               ,
    Z                XK_z               ,
    F1               XK_F1              ,
    F2               XK_F2              ,
    F3               XK_F3              ,
    F4               XK_F4              ,
    F5               XK_F5              ,
    F6               XK_F6              ,
    F7               XK_F7              ,
    F8               XK_F8              ,
    F9               XK_F9              ,
    F10              XK_F10             ,
    F11              XK_F11             ,
    F12              XK_F12             ,

    Esc              XK_Escape             ,
    Space            XK_space           ,
    Backspace        XK_BackSpace       ,
    Tab              XK_Tab             ,
    Enter            XK_Return           ,

    CapsLock         XK_Caps_Lock       ,
    NumLock          XK_Num_Lock        ,
    ScrollLock       XK_Scroll_Lock     ,

    Minus            XK_minus           ,
    Equal            XK_equal           ,
    LeftBrace        XK_braceleft     ,
    RightBrace       XK_braceright     ,
    Semicolon        XK_semicolon       ,
    Apostrophe       XK_apostrophe      ,
    Grave            XK_grave           ,
    Comma            XK_comma           ,
    Dot              XK_period          ,
    Slash            XK_slash           ,
    Backslash        XK_backslash       ,

    LCtrl            XK_Control_L          ,
    RCtrl            XK_Control_R          ,
    LShift           XK_Shift_L         ,
    RShift           XK_Shift_R         ,
    LAlt             XK_Alt_L           ,
    RAlt             XK_Alt_R           ,
    LSystem          XK_Super_L        ,
    RSystem          XK_Super_R        ,
    LMeta            XK_Meta_L         ,
    RMeta            XK_Meta_R         ,
    Compose          XK_Multi_key         ,

    Home             XK_Home            ,
    End              XK_End             ,

    Up               XK_Up              ,
    Down             XK_Down            ,
    Left             XK_Left            ,
    Right            XK_Right           ,

    PageUp           XK_Prior ,
    PageDown         XK_Next  ,

    Insert           XK_Insert          ,
    Delete           XK_Delete          ,

    SysRQ            XK_Sys_Req           ,
    LineFeed         XK_Linefeed        ,

    Kp0              XK_KP_0            ,
    Kp1              XK_KP_1            ,
    Kp2              XK_KP_2            ,
    Kp3              XK_KP_3            ,
    Kp4              XK_KP_4            ,
    Kp5              XK_KP_5            ,
    Kp6              XK_KP_6            ,
    Kp7              XK_KP_7            ,
    Kp8              XK_KP_8            ,
    Kp9              XK_KP_9            ,
    KpPlus           XK_KP_Add         ,
    KpMinus          XK_KP_Subtract        ,
    KpAsterisk       XK_KP_Multiply     ,
    KpSlash          XK_KP_Divide        ,
    KpDot            XK_KP_Decimal          ,
    KpEnter          XK_KP_Enter        ,
    KpEqual          XK_KP_Equal        ,
    KpComma          XK_KP_Separator        ,

    Mute             XF86XK_AudioMute            ,
    VolumeDown       XF86XK_AudioLowerVolume      ,
    VolumeUp         XF86XK_AudioRaiseVolume      ,
    Power            XF86XK_PowerOff           ,
    Pause            XK_Pause           ,

    ZenkakuHankaku   XK_Zenkaku_Hankaku  ,
    Katakana         XK_Katakana        ,
    Hiragana         XK_Hiragana        ,
    Henkan           XK_Henkan          ,
    KatakanaHiragana XK_Hiragana_Katakana,
    Muhenkan         XK_Muhenkan        ,

    /*
    Hangul           XK_Hangul         ,
    Hanja            XK_Hangul_Hanja           ,
    */
    Yen              XK_yen             ,
}




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
            fn fetch(x_display: *mut x::Display) -> Self {
                $(
                    let $atom = CString::new(stringify!($atom)).unwrap();
                    let $atom = unsafe { x::XInternAtom(
                        x_display, $atom.as_ptr(), x::False // Don't create
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

    // Motif
    _MOTIF_WM_HINTS

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



#[derive(Debug, Hash, PartialEq, Eq)]
pub struct XI {
    pub version: Semver,
    pub error_base: c_int,
    pub event_base: c_int,
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
pub struct XRender {
    pub version: Semver,
    pub error_base: c_int,
    pub event_base: c_int,
    pub argb32_pict_format: *mut xrender::XRenderPictFormat,
}

#[derive(Debug)]
pub struct SharedContext {
    pub x_display: *mut x::Display,
    pub atoms: PreparedAtoms,
    pub screen: *mut x::Screen, // NOTE: Nothing says it needs to be freed, so we don't.
    pub screen_num: c_int,
    pub root: x::Window,
    pub invisible_x_cursor: x::Cursor,
    pub glx: Option<Glx>,
    pub xi: Option<XI>,
    pub xim: Option<x::XIM>,
    pub xrender: Option<XRender>,
    pub usable_viewport: Rect<i32, u32>,
    pub previous_x_key_release_time: Cell<x::Time>,
    pub previous_x_key_release_keycode: Cell<c_uint>,
    pub previous_abs_mouse_position: Cell<Vec2<i32>>,
    pub weak_windows: RefCell<HashMap<x::Window, Weak<Window>>>, // NOTE: this wants VecMap instead, but it's unstable as of today.
    pub udev: *mut udev::udev,
    pub udev_monitor: *mut udev::udev_monitor,
}

#[derive(Debug)]
pub struct OsContext(Rc<SharedContext>);

#[derive(Debug)]
pub struct OsHid {
    pub shared_context: Rc<SharedContext>,
    pub udev_device: *mut udev::udev_device,
    pub fd: c_int,
    pub evdev: *mut evdev::libevdev,
    pub xi_devices: Vec<xi::XDevice>,
}
#[derive(Debug)]
pub struct OsWindow {
    pub shared_context: Rc<SharedContext>,
    pub x_window: x::Window,
    pub colormap: x::Colormap,
    pub glx_window: Option<GLXWindow>,
    pub xic: Option<x::XIC>,
    pub shows_cursor: Cell<bool>,
    pub user_cursor: RefCell<Option<Rc<Cursor>>>,
}
#[derive(Debug)]
pub struct OsGLContext {
    pub shared_context: Rc<SharedContext>,
    pub glx_context: GLXContext,
}
#[derive(Debug)]
pub struct OsGLPixelFormat {
    pub shared_context: Rc<SharedContext>,
    pub visual_info: *mut x::XVisualInfo,
    pub fbconfig: Option<GLXFBConfig>, // GLX >= 1.3
}
#[derive(Debug)]
pub struct OsCursor {
    pub shared_context: Rc<SharedContext>,
    pub x_cursor: x::Cursor,
    pub frames: Vec<xrender::XAnimCursor>,
}


impl Deref for OsContext {
    type Target = Rc<SharedContext>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for OsContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Dummy drop, so we don't write one by accident.
// The actual one is of course the one for SharedContext.
impl Drop for OsContext {
    fn drop(&mut self) {}
}

impl Drop for SharedContext {
    fn drop(&mut self) {
        unsafe {
            if let Some(xim) = self.xim {
                x::XCloseIM(xim);
            }
            x::XCloseDisplay(self.x_display);
            udev::udev_monitor_unref(self.udev_monitor);
            udev::udev_unref(self.udev);
        } 
    }
}

impl Drop for OsWindow {
    fn drop(&mut self) {
        let x_display = self.shared_context.x_display;
        unsafe {
            if let Some(xic) = self.xic {
                x::XDestroyIC(xic);
            }
            match self.glx_window {
                Some(w) => glXDestroyWindow(x_display, w),
                None => (),
            };
            x::XDestroyWindow(x_display, self.x_window);
            x::XFreeColormap(x_display, self.colormap);
        }
        self.shared_context.weak_windows.borrow_mut().remove(&self.x_window);
    }
}

impl Drop for OsHid {
    fn drop(&mut self) {
        unsafe {
            evdev::libevdev_free(self.evdev);
            libc::close(self.fd);
            udev::udev_device_unref(self.udev_device);
        }
    }
}

impl Drop for OsCursor {
    fn drop(&mut self) {
        unsafe {
            for frame in &self.frames {
                x::XFreeCursor(self.shared_context.x_display, frame.cursor);
            }
            x::XFreeCursor(self.shared_context.x_display, self.x_cursor);
        }
    }
}


impl Drop for OsGLPixelFormat {
    fn drop(&mut self) {
        unsafe {
            x::XFree(self.visual_info as *mut _); // NOTE: Fine to do on NULL.
        }
    }
}

impl Drop for OsGLContext {
    fn drop(&mut self) {
        let x_display = self.shared_context.x_display;
        unsafe {
            // Defers destruction until it's not current to any thread.
            glXDestroyContext(x_display, self.glx_context);
        }
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
            Some(r) => match r {
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


use context::Error;
use context::Error::*;

impl OsContext {
    pub fn open() -> Result<Self, Error> {
        Self::open_x11_display_name(None)
    }
    pub fn open_x11_display_name(x_display_name: Option<&CStr>) -> Result<Self, Error> {
        unsafe {
            // This thing is global to Xlib, and not inherent to X11.
            // We wouldn't have it if we used XCB.
            //
            // info!("Overriding process-wide Xlib error handler.");
            // x::XSetErrorHandler(Some(x_generic_error_handler));

            let x_display = x::XOpenDisplay(match x_display_name {
                Some(s) => {
                    info!("Opening X display {}", s.to_string_lossy());
                    s.as_ptr()
                },
                None => {
                    info!("Opening default X display");
                    ptr::null()
                }
            });
            if x_display.is_null() {
                return Err(Failed(match x_display_name {
                    None => "Failed to open default X display".to_owned(),
                    Some(name) => {
                        let name = name.to_string_lossy().into_owned();
                        format!("No X display named `{}`", name)
                    },
                }));
            }

            let protocol_version  = x::XProtocolVersion(x_display);
            let protocol_revision = x::XProtocolRevision(x_display);
            let screen_count      = x::XScreenCount(x_display);
            let vendor_release    = x::XVendorRelease(x_display);
            let display_string    = CStr::from_ptr(x::XDisplayString(x_display)).to_string_lossy();
            let server_vendor     = CStr::from_ptr(x::XServerVendor(x_display) ).to_string_lossy();
            info!("Opened X11 display `{}`", display_string);
            info!("X Protocol version {}, revision {}", protocol_version, protocol_revision);
            info!("Vendor: `{}`, release {}", server_vendor, vendor_release);
            info!("Screen count: {}", screen_count);

            let screen = x::XDefaultScreenOfDisplay(x_display);
            let screen_num = x::XDefaultScreen(x_display);
            let root = x::XDefaultRootWindow(x_display);
            let atoms = PreparedAtoms::fetch(x_display);

            let xim = unsafe {
                let xim = x::XOpenIM(
                    x_display, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()
                );
                if xim.is_null() {
                    None
                } else {
                    Some(xim)
                }
            };
            let glx = Self::query_glx(x_display, screen_num);
            let xi = Self::query_xi(x_display, screen_num);
            let xrender = Self::query_xrender(x_display, screen_num);
            let usable_viewport = Self::query_usable_viewport(x_display, screen_num, &atoms);
            let weak_windows = RefCell::new(Default::default());
            let udev = udev::udev_new();
            let udev_monitor = udev::udev_monitor_new_from_netlink(udev, b"udev\0" as *const _ as _);
            let _status = udev::udev_monitor_enable_receiving(udev_monitor);
            let invisible_x_cursor = {
                let data = 0 as c_char;
                let mut col: x::XColor = mem::zeroed();
                let pix = x::XCreateBitmapFromData(x_display, root, &data, 1, 1);
                let cur = x::XCreatePixmapCursor(x_display, pix, pix, &mut col, &mut col, 0, 0);
                x::XFreePixmap(x_display, pix);
                cur
            };
            let shared_context = SharedContext { 
                x_display, atoms, screen, screen_num, root, glx, xi, xim, xrender,
                udev, udev_monitor,
                usable_viewport,
                previous_x_key_release_time: Cell::new(0),
                previous_x_key_release_keycode: Cell::new(0),
                previous_abs_mouse_position: Cell::new(Default::default()),
                weak_windows, invisible_x_cursor,
            };
            let shared_context = Rc::new(shared_context);
            Ok(OsContext(shared_context))
        }
    }

    // FIXME: If we can't, return the whole screen size.
    fn query_usable_viewport(x_display: *mut x::Display, screen_num: c_int, atoms: &PreparedAtoms) -> Rect<i32, u32> {
        let mut real_type: x::Atom = 0;
        let mut real_format: c_int = 0;
        let mut items_read: c_ulong = 0;
        let mut items_left: c_ulong = 0;
        let mut propdata: *mut c_uchar = ptr::null_mut();

        // XXX: Dubious
        let fallback = unsafe { Rect {
            x: 0, y: 0,
            w: x::XDisplayWidth(x_display, screen_num) as u32,
            h: x::XDisplayHeight(x_display, screen_num) as u32,
        }};

        let status = unsafe { x::XGetWindowProperty(
            x_display, x::XDefaultRootWindow(x_display), atoms._NET_WORKAREA,
            0, 4, x::False, x::XA_CARDINAL,
            &mut real_type, &mut real_format, &mut items_read,
            &mut items_left, &mut propdata
        )};
        let usable = if status == x::Success as _ && items_read >= 4 {
            let p = unsafe {
                slice::from_raw_parts(propdata as *const c_long, items_read as _)
            };
            let usable = Rect {
                x: p[0] as _, y: p[1] as _, w: p[2] as _, h: p[3] as _
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

    fn query_xi(x_display: *mut x::Display, screen_num: c_int) -> Option<XI> {

        let iname: *const c_char = b"XInputExtension\0" as *const _ as _;
        let (error_base, event_base, xi_opcode) = unsafe {
            let (mut error_base, mut event_base, mut xi_opcode) = mem::uninitialized();
            let has_xi2 = x::XQueryExtension(
                x_display, iname,
                &mut xi_opcode, &mut event_base, &mut error_base
            );
            if has_xi2 == x::False {
                return None;
            }
            (error_base, event_base, xi_opcode)
        };
        unsafe {
            G_XI_ERROR_BASE = error_base;
            G_XI_EVENT_BASE = event_base;
            G_XI_OPCODE = xi_opcode;
        }
        let version = unsafe {
            xi::XGetExtensionVersion(x_display, iname)
        };
        let version = if !version.is_null() && version != xxi::NoSuchExtension as _ {
            unsafe {
                let major = (*version).major_version;
                let minor = (*version).minor_version;
                x::XFree(version as _);
                info!("XInput extension version is {}.{}", major, minor);
                Semver::new(major as _, minor as _, 0)
            }
        } else {
            return None;
        };
        info!("XInput error_base = {}, event_base = {}", error_base, event_base);
        let mut major = 2;
        let mut minor = 2;
        unsafe {
            xi2::XIQueryVersion(x_display, &mut major, &mut minor);
        }


        /*
        XIEventMask mask[2];
        XIEventMask *m;
        m = &mask[0];
        m->deviceid = (deviceid == -1) ? XIAllDevices : deviceid;
        m->mask_len = XIMaskLen(XI_LASTEVENT);
        m->mask = calloc(m->mask_len, sizeof(char));
        XISetMask(m->mask, XI_ButtonPress);
        XISetMask(m->mask, XI_ButtonRelease);
        XISetMask(m->mask, XI_KeyPress);
        XISetMask(m->mask, XI_KeyRelease);
        XISetMask(m->mask, XI_Motion);
        XISetMask(m->mask, XI_DeviceChanged);
        XISetMask(m->mask, XI_Enter);
        XISetMask(m->mask, XI_Leave);
        XISetMask(m->mask, XI_FocusIn);
        XISetMask(m->mask, XI_FocusOut);
        XISetMask(m->mask, XI_TouchBegin);
        XISetMask(m->mask, XI_TouchUpdate);
        XISetMask(m->mask, XI_TouchEnd);

        if (m->deviceid == XIAllDevices)
            XISetMask(m->mask, XI_HierarchyChanged);
        XISetMask(m->mask, XI_PropertyEvent);

        m = &mask[1];
        m->deviceid = (deviceid == -1) ? XIAllMasterDevices : deviceid;
        m->mask_len = XIMaskLen(XI_LASTEVENT);
        m->mask = calloc(m->mask_len, sizeof(char));
        XISetMask(m->mask, XI_RawKeyPress);
        XISetMask(m->mask, XI_RawKeyRelease);
        XISetMask(m->mask, XI_RawButtonPress);
        XISetMask(m->mask, XI_RawButtonRelease);
        XISetMask(m->mask, XI_RawMotion);
        XISetMask(m->mask, XI_RawTouchBegin);
        XISetMask(m->mask, XI_RawTouchUpdate);
        XISetMask(m->mask, XI_RawTouchEnd);

        XISelectEvents(display, win, &mask[0], use_root ? 2 : 1);
        if (!use_root) {
            XISelectEvents(display, DefaultRootWindow(display), &mask[1], 1);
            XMapWindow(display, win);
        }
        XSync(display, False);

        free(mask[0].mask);
        free(mask[1].mask);

        XEvent event;
        XMaskEvent(display, ExposureMask, &event);
        XSelectInput(display, win, 0);
        */
        Some(XI { event_base, error_base, version })
        /*
        XGetExtensionVersion();
        XIQueryVersion();
        XIQueryDevice();
        XIFreeDeviceInfo();
        XSelectInput();
        XISelectEvents();
        XNextEvent();
        XGetEventData();
        XFreeEventData();
        */
    }


    fn query_xrender(x_display: *mut x::Display, screen_num: c_int) -> Option<XRender> {
        let (error_base, event_base) = unsafe {
            let (mut error_base, mut event_base) = mem::uninitialized();
            let has_it = xrender::XRenderQueryExtension(x_display, &mut error_base, &mut event_base);
            if has_it == x::False {
                return None;
            }
            (error_base, event_base)
        };
        unsafe {
            G_XRENDER_ERROR_BASE = error_base;
            G_XRENDER_EVENT_BASE = event_base;
        }
        info!("XRender error_base = {}, event_base = {}", error_base, event_base);

        let (major, minor) = unsafe {
            let (mut major, mut minor) = (1, 0);
            let success = xrender::XRenderQueryVersion(x_display, &mut major, &mut minor);
            if success == x::False {
               return None;
            }
            (major as u32, minor as u32)
        };
        let version = Semver::new(major, minor, 0);
        info!("XRender extension version {}.{}", major, minor);

        let argb32_pict_format = unsafe {
            xrender::XRenderFindStandardFormat(
                x_display, xxrender::PictStandard::ARGB32 as _
            )
        };
        Some(XRender { version, error_base, event_base, argb32_pict_format })
    }

    fn query_glx(x_display: *mut x::Display, screen_num: c_int) -> Option<Glx> {

        let (error_base, event_base) = unsafe {
            let (mut error_base, mut event_base) = mem::uninitialized();
            let has_glx = glXQueryExtension(x_display, &mut error_base, &mut event_base);
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
            let success = glXQueryVersion(x_display, &mut major, &mut minor);
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

        Some(Glx { version, get_proc_address, ext, error_base, event_base })
    }

    pub(crate) fn add_weak_window(&mut self, strong: &Rc<Window>) {
        let weak = Rc::downgrade(strong);
        self.weak_windows.borrow_mut().insert(strong.os_window.x_window, weak);
    }


    pub fn create_window(&mut self, settings: &WindowSettings) -> Result<OsWindow, Error> {
        let x_display = self.x_display;
        let parent = unsafe { x::XDefaultRootWindow(x_display) };
        
        let &WindowSettings {
            mode, resizable, fully_opaque, ref opengl, allow_high_dpi
        } = settings;

        let _ = allow_high_dpi;
        let _ = fully_opaque;

        let (visual, depth, colormap) = match *opengl {
            Some(ref pixel_format) => {
                if self.glx.is_none() {
                    return Err(Failed("Cannot create OpenGL-capable window without GLX".to_owned()));
                }
                let vi = unsafe { *pixel_format.0.visual_info };
                let colormap = unsafe {
                    x::XCreateColormap(x_display, parent, vi.visual, x::AllocNone)
                };
                (vi.visual, vi.depth, colormap)
            },
            None => {
                let depth = x::CopyFromParent;
                let visual = unsafe {
                    x::XDefaultVisual(x_display, self.screen_num)
                };
                let colormap = unsafe {
                    x::XCreateColormap(x_display, parent, visual, x::AllocNone)
                };
                (visual, depth, colormap)
            },
        };

        let (w, h, maximized, fullscreen) = match mode {
            WindowMode::FixedSize(Extent2 { w, h }) => (w, h, false, false),
            // FIXME: Don't give `1` as extents. The accuracy is relied upon later.
            WindowMode::Maximized => (1, 1, true, false),
            WindowMode::FullScreen => (1, 1, false, true),
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
            x_display, parent, x, y, w, h,
            border_thickness, depth, class as _, visual, valuemask, &mut swa
        )};

        if x_window == 0 {
            return Err(Failed("XCreateWindow() failed".to_owned()));
        }

        unsafe {
            let mut protocols = [ 
                self.atoms.WM_DELETE_WINDOW,
                self.atoms._NET_WM_PING,
                self.atoms.WM_TAKE_FOCUS,
            ];
            x::XSetWMProtocols(
                x_display, x_window, protocols.as_mut_ptr(), protocols.len() as _
            );

            let pid = libc::getpid();
            if pid > 0 {
                x::XChangeProperty(
                    x_display, x_window, self.atoms._NET_WM_PID, 
                    x::XA_CARDINAL, 32, x::PropModeReplace,
                    &pid as *const _ as *const _, 
                    1
                );
            }
            /*
            x::XChangeProperty(
                x_display, x_window, self.atoms.XdndAware, 
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
            flags: x::InputHint, //| x::StateHint, //| x::WindowGroupHint | x::IconPixmapHint | ...
            input: x::True,
            // initial_state: x::NormalState,
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
            // replaces x::XSetWMNormalHints(x_display, x_window, &mut hints);
            x::XSetWMProperties(
                x_display, x_window, window_name, icon_name, argv, argc,
                sizehints_buf, wmhints_buf, classhint_buf
            );
            x::XFree(sizehints_buf as _);
            x::XFree(classhint_buf as _);
            x::XFree(wmhints_buf as _);

            let always_on_top = false;
            let skip_taskbar = false;
            let input_focus = true;
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
                    x_display, x_window, self.atoms._NET_WM_STATE, x::XA_ATOM, 32,
                    x::PropModeReplace, atoms.as_mut_ptr() as *mut _, count as _
                );
            } else {
                x::XDeleteProperty(x_display, x_window, self.atoms._NET_WM_STATE);
            }

            // TODO: There are many other possible types of window.
            let mut wintype = self.atoms._NET_WM_WINDOW_TYPE_NORMAL;
            x::XChangeProperty(
                x_display, x_window, self.atoms._NET_WM_WINDOW_TYPE, x::XA_ATOM, 32,
                x::PropModeReplace, &mut wintype as *mut _ as *mut _, 1
            );

            // TODO: Test this.
            // Raise the window so that it appears on top of the stack when it
            // is shown
            x::XRaiseWindow(x_display, x_window);
        }

        let wants_glx_window = {
            opengl.is_some() && self.glx.as_ref().unwrap().version >= Semver::new(1,3,0)
        };

        let glx_window = if wants_glx_window {
            let fbconfig = opengl.as_ref().unwrap().0.fbconfig.unwrap();
            Some(unsafe { glXCreateWindow(
                x_display, fbconfig, x_window, ptr::null_mut()
            )})
        } else { None };

        let xic = unsafe {
            if let Some(xim) = self.xim {
                let xic = x::XCreateIC(xim, 
                    x::XNClientWindow, x_window,
                    x::XNFocusWindow, x_window,
                    x::XNInputStyle, x::XIMPreeditNothing | x::XIMStatusNothing,
                    ptr::null_mut() as *mut c_void,
                );
                if xic.is_null() {
                    None
                } else {
                    Some(xic)
                }
            } else {
                None
            }
        };

        Ok(OsWindow { 
            shared_context: self.0.clone(), x_window, colormap, glx_window,
            xic, shows_cursor: Cell::new(true), user_cursor: RefCell::new(None),
        })
    }
    pub fn choose_gl_pixel_format(&self, settings: &GLPixelFormatSettings) -> Result<OsGLPixelFormat, Error> {
        let x_display = self.x_display;

        let glx = match self.glx.as_ref() {
            None => return Err(Failed("The GLX extension is not present".to_owned())),
            Some(glx) => glx,
        };

        if glx.version < Semver::new(1,3,0) {
            // Not actually mutated, but glXChooseVisual wants *mut...
            let mut visual_attribs = glx.gen_visual_attribs(settings);
            let visual_info = unsafe { glXChooseVisual(
                x_display, self.screen_num, visual_attribs.as_mut_ptr()
            )};
            if visual_info.is_null() {
                return Err(Failed("glXChooseVisual() failed".to_owned()));
            }
            return Ok(OsGLPixelFormat { 
                shared_context: self.0.clone(),
                visual_info, fbconfig: None
            });
        }

        // If we're here, we have GLX >= 1.3.

        let visual_attribs = glx.gen_fbconfig_attribs(settings);
        let mut fbcount: c_int = 0;
        let fbcs = unsafe { glXChooseFBConfig(
            x_display, self.screen_num, visual_attribs.as_ptr(), &mut fbcount
        )};
        if fbcs.is_null() || fbcount == 0 {
            return Err(Failed("No matching FBConfig was found!".to_owned()));
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
                glXGetVisualFromFBConfig(x_display, fbc)
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
                glXGetFBConfigAttrib(x_display, fbc, GLX_SAMPLE_BUFFERS         , &mut sample_buffers         );
                glXGetFBConfigAttrib(x_display, fbc, GLX_SAMPLES                , &mut samples                );
                glXGetFBConfigAttrib(x_display, fbc, GLX_FBCONFIG_ID            , &mut fbconfig_id            );
                glXGetFBConfigAttrib(x_display, fbc, GLX_BUFFER_SIZE            , &mut buffer_size            );
                glXGetFBConfigAttrib(x_display, fbc, GLX_LEVEL                  , &mut level                  );
                glXGetFBConfigAttrib(x_display, fbc, GLX_DOUBLEBUFFER           , &mut stereo                 );
                glXGetFBConfigAttrib(x_display, fbc, GLX_STEREO                 , &mut doublebuffer           );
                glXGetFBConfigAttrib(x_display, fbc, GLX_AUX_BUFFERS            , &mut aux_buffers            );
                glXGetFBConfigAttrib(x_display, fbc, GLX_RED_SIZE               , &mut red_size               );
                glXGetFBConfigAttrib(x_display, fbc, GLX_GREEN_SIZE             , &mut green_size             );
                glXGetFBConfigAttrib(x_display, fbc, GLX_BLUE_SIZE              , &mut blue_size              );
                glXGetFBConfigAttrib(x_display, fbc, GLX_ALPHA_SIZE             , &mut alpha_size             );
                glXGetFBConfigAttrib(x_display, fbc, GLX_DEPTH_SIZE             , &mut depth_size             );
                glXGetFBConfigAttrib(x_display, fbc, GLX_STENCIL_SIZE           , &mut stencil_size           );
                glXGetFBConfigAttrib(x_display, fbc, GLX_ACCUM_RED_SIZE         , &mut accum_red_size         );
                glXGetFBConfigAttrib(x_display, fbc, GLX_ACCUM_GREEN_SIZE       , &mut accum_green_size       );
                glXGetFBConfigAttrib(x_display, fbc, GLX_ACCUM_BLUE_SIZE        , &mut accum_blue_size        );
                glXGetFBConfigAttrib(x_display, fbc, GLX_ACCUM_ALPHA_SIZE       , &mut accum_alpha_size       );
                glXGetFBConfigAttrib(x_display, fbc, GLX_RENDER_TYPE            , &mut render_type            );
                glXGetFBConfigAttrib(x_display, fbc, GLX_DRAWABLE_TYPE          , &mut drawable_type          );
                glXGetFBConfigAttrib(x_display, fbc, GLX_X_RENDERABLE           , &mut x_renderable           );
                glXGetFBConfigAttrib(x_display, fbc, GLX_VISUAL_ID              , &mut visual_id              );
                glXGetFBConfigAttrib(x_display, fbc, GLX_X_VISUAL_TYPE          , &mut x_visual_type          );
                glXGetFBConfigAttrib(x_display, fbc, GLX_CONFIG_CAVEAT          , &mut config_caveat          );
                glXGetFBConfigAttrib(x_display, fbc, GLX_TRANSPARENT_TYPE       , &mut transparent_type       );
                glXGetFBConfigAttrib(x_display, fbc, GLX_TRANSPARENT_INDEX_VALUE, &mut transparent_index_value);
                glXGetFBConfigAttrib(x_display, fbc, GLX_TRANSPARENT_RED_VALUE  , &mut transparent_red_value  );
                glXGetFBConfigAttrib(x_display, fbc, GLX_TRANSPARENT_GREEN_VALUE, &mut transparent_green_value);
                glXGetFBConfigAttrib(x_display, fbc, GLX_TRANSPARENT_BLUE_VALUE , &mut transparent_blue_value );
                glXGetFBConfigAttrib(x_display, fbc, GLX_TRANSPARENT_ALPHA_VALUE, &mut transparent_alpha_value);
                glXGetFBConfigAttrib(x_display, fbc, GLX_MAX_PBUFFER_WIDTH      , &mut max_pbuffer_width      );
                glXGetFBConfigAttrib(x_display, fbc, GLX_MAX_PBUFFER_HEIGHT     , &mut max_pbuffer_height     );
                glXGetFBConfigAttrib(x_display, fbc, GLX_MAX_PBUFFER_PIXELS     , &mut max_pbuffer_pixels     );
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
            let visual_info = glXGetVisualFromFBConfig(x_display, best_fbc);
            assert!(!visual_info.is_null());
            Ok(OsGLPixelFormat {
                shared_context: self.0.clone(),
                visual_info, fbconfig: Some(best_fbc)
            })
        }
    }
    pub fn create_gl_context(&self, pf: &OsGLPixelFormat, cs: &GLContextSettings) -> Result<OsGLContext, Error> {
        let x_display = self.x_display;

        let glx = match self.glx.as_ref() {
            None => return Err(Failed("Creating an OpenGL context requires GLX".to_owned())),
            Some(glx) => glx,
        };

        let &OsGLPixelFormat { visual_info, fbconfig, .. } = pf;

        unsafe {
            x::XSync(x_display, x::False);
            G_XLIB_ERROR_OCCURED.store(false, Ordering::SeqCst);
        }

        let (funcname, glx_context) = unsafe {
            if glx.version < Semver::new(1,3,0) {
                ("glXCreateContext", glXCreateContext(x_display, visual_info, ptr::null_mut(), x::True))
            } else if glx.version < Semver::new(1,4,0) 
                   || (glx.version >= Semver::new(1,4,0) && !glx.ext.GLX_ARB_create_context)
            {
                ("glXCreateNewContext", glXCreateNewContext(
                    x_display, fbconfig.unwrap(), GLX_RGBA_TYPE, ptr::null_mut(), x::True
                ))
            } else {
                #[allow(non_snake_case)]
                let glXCreateContextAttribsARB = glx.ext.glXCreateContextAttribsARB.unwrap();
                let attribs_arb = glx.gen_arb_attribs(cs);
                ("glxCreateContextAttribsARB", glXCreateContextAttribsARB(
                    x_display, fbconfig.unwrap(), ptr::null_mut(), x::True, attribs_arb.as_ptr()
                ))
            }
        };

        unsafe {
            x::XSync(x_display, x::False);
            if glx_context.is_null() || G_XLIB_ERROR_OCCURED.load(Ordering::SeqCst) {
                return Err(Failed(format!("{}() failed", funcname)));
            }

            info!("GLX context is direct: {}", glXIsDirect(x_display, glx_context));
            Ok(OsGLContext { shared_context: self.0.clone(), glx_context })
        }

    }
    pub fn create_software_gl_context(&self, pf: &OsGLPixelFormat, cs: &GLContextSettings) -> Result<OsGLContext, Error> {
        Err(Unsupported(Some("This is not implemented yet! The plan is to load the Mesa driver, if present".to_owned())))
    }
    pub fn create_gl_context_from_lib(&self, _pf: &OsGLPixelFormat, _cs: &GLContextSettings, _path: &Path) -> Result<OsGLContext, Error> {
        Err(Unsupported(Some("This is not implemented yet!".to_owned())))
    }

    // WISH: The proper way is to install a signal handler which catch SIGTERM.
    // See https://stackoverflow.com/a/22009848
    pub fn allow_session_termination(&mut self) -> Result<(), Error> {
        Err(Error::Unsupported(Some(Self::unsupported_sigterm().to_owned())))
    }
    pub fn disallow_session_termination(&mut self, _reason: Option<String>) -> Result<(), Error> {
        Err(Error::Unsupported(Some(Self::unsupported_sigterm().to_owned())))
    }
    fn unsupported_sigterm() -> &'static str {
        "Detecting session termination on Linux requires handling SIGTERM, which is an intrusive operation and therefore not implemented right now."
    }


    pub fn query_best_cursor_size(&self, size_hint: Extent2<u32>) -> Extent2<u32> {
        let mut best_w: c_uint = 0;
        let mut best_h: c_uint = 0;
        unsafe {
            let drawable = x::XDefaultRootWindow(self.x_display);
            x::XQueryBestCursor(
                self.x_display, drawable, size_hint.w as _, size_hint.h as _,
                &mut best_w, &mut best_h
            );
        }
        Extent2 { w: best_w as _, h: best_h as _ }
    }
    fn cursordata_to_x_cursor(&self, xrender: &XRender, frame: &CursorData) -> x::Cursor {
        unsafe {
            let dpy = self.x_display;
            let root = self.root;
            let Extent2 { w, h } = frame.image.size;
            let visual = x::XDefaultVisual(dpy, self.screen_num);
            let pix = x::XCreatePixmap(dpy, root, w, h, 32);
            let pix_gc = x::XCreateGC(dpy, pix, 0, ptr::null_mut());
            let pix_img = x::XCreateImage(
                dpy, visual, 32, x::ZPixmap, 0,
                frame.image.pixels.as_ptr() as *const _ as *mut _,
                w, h, 32, 4*(w as c_int)
            );
            x::XPutImage(dpy, pix, pix_gc, pix_img, 0, 0, 0, 0, w, h);
            let pic_format = xrender.argb32_pict_format;
            let pic = xrender::XRenderCreatePicture(dpy, pix, pic_format, 0, ptr::null_mut());
            let Vec2 { x, y } = frame.hotspot;
            let x_cursor = xrender::XRenderCreateCursor(dpy, pic, x as _, y as _);
            xrender::XRenderFreePicture(dpy, pic);
            x::XDestroyImage(pix_img);
            x::XFreeGC(dpy, pix_gc);
            x::XFreePixmap(dpy, pix);
            x_cursor
        }
    }
    pub fn create_rgba32_cursor(&self, frame: CursorData) -> Result<OsCursor, Error> {
        let xrender = match self.xrender.as_ref() {
            Some(x) => x,
            None => return Err(Failed("Creating an RGBA cursor requires the XRender extension".to_owned())),
        };
        let x_cursor = self.cursordata_to_x_cursor(xrender, &frame);
        Ok(OsCursor { shared_context: self.0.clone(), x_cursor, frames: vec![] })
    }
    pub fn create_animated_rgba32_cursor(&self, anim: &[CursorFrame]) -> Result<OsCursor, Error> {
        let xrender = match self.xrender.as_ref() {
            Some(x) => x,
            None => return Err(Failed("Creating an animated RGBA cursor requires the XRender extension".to_owned())),
        };
        let mut frames: Vec<_> = anim.iter().map(|frame| {
            let secs = frame.duration.as_secs() as u64;
            let nano = frame.duration.subsec_nanos() as u64;
            let delay = secs*1000 + nano/1_000_000;
            let cursor = self.cursordata_to_x_cursor(xrender, &frame.data);
            xrender::XAnimCursor { cursor, delay }
        }).collect();
        let x_cursor = unsafe {
            xrender::XRenderCreateAnimCursor(self.x_display, frames.len() as _, frames.as_mut_ptr())
        };
        Ok(OsCursor { shared_context: self.0.clone(), x_cursor, frames })
    }
    pub fn create_system_cursor(&self, s: SystemCursor) -> Result<OsCursor, Error> {
        let glyph = match s {
            SystemCursor::Arrow => XC_left_ptr,
            SystemCursor::Hand => XC_hand2,
            SystemCursor::Ibeam => XC_xterm,
            SystemCursor::Wait => XC_watch,
            SystemCursor::Crosshair => XC_tcross,
            SystemCursor::WaitArrow => XC_watch,
            SystemCursor::ResizeNWToSE => XC_fleur,
            SystemCursor::ResizeNEToSW => XC_fleur,
            SystemCursor::ResizeV => XC_sb_v_double_arrow,
            SystemCursor::ResizeH => XC_sb_h_double_arrow,
            SystemCursor::ResizeHV => XC_fleur,
            SystemCursor::Deny => XC_pirate,
            SystemCursor::Question => XC_question_arrow,
            SystemCursor::ReverseArrow => XC_right_ptr,
            SystemCursor::TopSide => XC_top_side,
            SystemCursor::BottomSide => XC_bottom_side,
            SystemCursor::LeftSide => XC_left_side,
            SystemCursor::RightSide => XC_right_side,
            SystemCursor::BottomLeftCorner => XC_bottom_left_corner,
            SystemCursor::BottomRightCorner => XC_bottom_right_corner,
            SystemCursor::TopLeftCorner => XC_top_left_corner,
            SystemCursor::TopRightCorner => XC_top_right_corner,
            SystemCursor::Pencil => XC_pencil,
            SystemCursor::Spraycan => XC_spraycan,
        };
        let x_cursor = unsafe {
            x::XCreateFontCursor(self.x_display, glyph)
        };
        Ok(OsCursor { shared_context: self.0.clone(), x_cursor, frames: vec![] })
    }

    // TODO: udev_monitor_receive_device()
    pub fn poll_next_event(&mut self) -> Option<Event> {
        unsafe {
            let x_display = self.x_display;
            let mut x_event: x::XEvent = mem::uninitialized();
            x::XFlush(x_display);
            loop {
                let event_count = x::XPending(x_display);
                if event_count <= 0 {
                    return None;
                }
                x::XNextEvent(x_display, &mut x_event);
                let e = self.x_event_to_event(x_event);
                if let Ok(e) = e {
                    return Some(e);
                }
            }
        }
    }
    pub fn wait_next_event(&mut self, timeout: Timeout) -> Option<Event> {
        unsafe {
            let x_display = self.x_display;
            let mut x_event: x::XEvent = mem::uninitialized();
            match timeout {
                Timeout::Infinite => loop {
                    x::XNextEvent(x_display, &mut x_event);
                    let e = self.x_event_to_event(x_event);
                    if let Ok(e) = e {
                        return Some(e);
                    }
                },
                Timeout::Set(duration) => {
                    // Welp, just poll repeatedly instead
                    let now = Instant::now();
                    while now.elapsed() < duration {
                        let event_count = x::XPending(x_display);
                        if event_count <= 0 {
                            continue;
                        }
                        x::XNextEvent(x_display, &mut x_event);
                        let e = self.x_event_to_event(x_event);
                        if let Ok(e) = e {
                            return Some(e);
                        }
                    }
                },
            };
            unreachable!{}
        }
    }
    fn x_window_to_window(&self, x_window: x::Window) -> Option<Rc<Window>> {
        let weak_windows = self.weak_windows.borrow();
        let weak = weak_windows.get(&x_window)?;
        weak.upgrade()
    }
    fn dummy_keyboard(&self) -> Rc<Keyboard> {
        unimplemented!{}
    }
    fn dummy_mouse(&self) -> Rc<Mouse> {
        unimplemented!{}
    }
    fn x_key_event_to_key(&self,  x_event: &x::XKeyEvent) -> Key {
        let keysym = unsafe {
            x::XLookupKeysym(x_event as *const _ as *mut _, 0)
        };
        self.x_keysym_to_key(keysym)
    }
    fn x_keysym_to_key(&self, x_keysym: x::KeySym) -> Key {
        unimplemented!{}
    }

    fn xi_event_to_event(&mut self, cookie: &x::XGenericEventCookie) -> Result<Event, ()> {
        match cookie.evtype {
            // XIDeviceChangedEvent
            xi2::XI_DeviceChanged   => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIDeviceChangedEvent) };
                /*
                    pub time: Time,
                        pub deviceid: c_int,
                            pub sourceid: c_int,
                                pub reason: c_int,
                                    pub num_classes: c_int,
                                        pub classes: *mut *mut XIAnyClassInfo,
                                        */
                warn!("XI event: {}", "DeviceChanged"    );
                Err(())
            },

            // XIHierarchyEvent
            xi2::XI_HierarchyChanged=> {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIHierarchyEvent) };
                /*
                    pub time: Time,
                        pub flags: c_int,
                            pub num_info: c_int,
                                pub info: *mut XIHierarchyInfo,
                                */
                warn!("XI event: {}", "HierarchyChanged" );
                Err(())
            },

            // XIEnterEvent
            xi2::XI_Enter           => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIEnterEvent) };
                /*
                    pub time: Time,
                        pub deviceid: c_int,
                            pub sourceid: c_int,
                                pub detail: c_int,
                                    pub root: Window,
                                        pub event: Window,
                                            pub child: Window,
                                                pub root_x: c_double,
                                                    pub root_y: c_double,
                                                        pub event_x: c_double,
                                                            pub event_y: c_double,
                                                                pub mode: c_int,
                                                                    pub focus: c_int,
                                                                        pub same_screen: c_int,
                                                                            pub buttons: XIButtonState,
                                                                                pub mods: XIModifierState,
                                                                                    pub group: XIGroupState,
                                                                                    */
                warn!("XI event: {}", "Enter"            );
                Err(())
            },
            xi2::XI_Leave           => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIEnterEvent) };
                warn!("XI event: {}", "Leave"            );
                Err(())
            },
            xi2::XI_FocusIn         => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIEnterEvent) };
                warn!("XI event: {}", "FocusIn"          );
                Err(())
            },
            xi2::XI_FocusOut        => { 
                let x_event = unsafe { &*(cookie.data as *const xi2::XIEnterEvent) };
                warn!("XI event: {}", "FocusOut"         );
                Err(())
            },

            // XIPropertyEvent
            xi2::XI_PropertyEvent   => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIPropertyEvent) };
                /*
                    pub time: Time,
                        pub deviceid: c_int,
                            pub property: Atom,
                                pub what: c_int,
                                */
                warn!("XI event: {}", "PropertyEvent"    );
                Err(())
            },

            // XIDeviceEvent
            xi2::XI_KeyPress        => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIDeviceEvent) };
                let &xi2::XIDeviceEvent {
                    time, deviceid, sourceid, detail, root,
                    event: x_window, child, root_x, root_y,
                    event_x, event_y, flags, buttons, valuators, mods, group,
                    ..
                } = x_event;
                let window = self.x_window_to_window(x_window);
                if let Some(window) = window.as_ref() {
                    window.os_window.set_net_wm_user_time(time);
                }
                warn!("XI event: {}", "KeyPress"         );
                Err(())
            },
            xi2::XI_KeyRelease      => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIDeviceEvent) };
                warn!("XI event: {}", "KeyRelease"       );
                Err(())
            },
            xi2::XI_ButtonPress     => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIDeviceEvent) };
                // event.window.set_net_wm_user_time(x_event.time);
                warn!("XI event: {}", "ButtonPress"      );
                Err(())
            },
            xi2::XI_ButtonRelease   => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIDeviceEvent) };
                warn!("XI event: {}", "ButtonRelease"    );
                Err(())
            },
            xi2::XI_Motion          => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIDeviceEvent) };
                warn!("XI event: {}", "Motion"           );
                Err(())
            },
            xi2::XI_TouchBegin      => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIDeviceEvent) };
                // event.window.set_net_wm_user_time(x_event.time);
                warn!("XI event: {}", "TouchBegin"       );
                Err(())
            },
            xi2::XI_TouchUpdate     => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIDeviceEvent) };
                warn!("XI event: {}", "TouchUpdate"      );
                Err(())
            },
            xi2::XI_TouchEnd        => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIDeviceEvent) };
                warn!("XI event: {}", "TouchEnd"         );
                Err(())
            },

            // XIRawEvent
            xi2::XI_RawKeyPress     => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIRawEvent) };
                /*
                    pub time: Time,
                        pub deviceid: c_int,
                            pub sourceid: c_int,
                                pub detail: c_int,
                                    pub flags: c_int,
                                        pub valuators: XIValuatorState,
                                            pub raw_values: *mut c_double,
                                            */
                warn!("XI event: {}", "RawKeyPress"      );
                Err(())
            },
            xi2::XI_RawKeyRelease   => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIRawEvent) };
                warn!("XI event: {}", "RawKeyRelease"    );
                Err(())
            },
            xi2::XI_RawButtonPress  => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIRawEvent) };
                warn!("XI event: {}", "RawButtonPress"   );
                Err(())
            },
            xi2::XI_RawButtonRelease=> {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIRawEvent) };
                warn!("XI event: {}", "RawButtonRelease" );
                Err(())
            },
            xi2::XI_RawMotion       => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIRawEvent) };
                warn!("XI event: {}", "RawMotion"        );
                Err(())
            },
            xi2::XI_RawTouchBegin   => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIRawEvent) };
                warn!("XI event: {}", "RawTouchBegin"    );
                Err(())
            },
            xi2::XI_RawTouchUpdate  => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIRawEvent) };
                warn!("XI event: {}", "RawTouchUpdate"   );
                Err(())
            },
            xi2::XI_RawTouchEnd     => {
                let x_event = unsafe { &*(cookie.data as *const xi2::XIRawEvent) };
                warn!("XI event: {}", "RawTouchEnd"      );
                Err(())
            },
            _ => {
                warn!("Unrecognized XI event: {:?}", cookie);
                Err(())
            }
        }
    }
    fn x_utf8_lookup_string(&self, xic: x::XIC, x_event: &x::XKeyEvent) -> (Option<x::KeySym>, Option<String>) {
        // Asserting because of undefined behaviour otherwise.
        debug_assert_ne!(x_event.type_, x::KeyRelease);
        unsafe {
            let mut buf: Vec<u8> = Vec::with_capacity(32);
            let mut keysym: x::KeySym = 0;
            let mut status: x::Status = 0;
            loop {
                let actual_len = x::Xutf8LookupString(
                    xic, x_event as *const _ as *mut _,
                    buf.as_mut_ptr() as _, buf.capacity() as _,
                    &mut keysym, &mut status
                );
                match status {
                    x::XBufferOverflow => {
                        buf.reserve_exact(actual_len as _);
                        continue;
                    },
                    x::XLookupNone => return (None, None),
                    x::XLookupKeySym => return (Some(keysym), None),
                    x::XLookupChars => (),
                    x::XLookupBoth => (),
                    _ => unreachable!{},
                };
                buf.set_len(actual_len as _);
                let s = String::from_utf8(buf).unwrap();
                match status {
                    x::XLookupChars => return (None, Some(s)),
                    x::XLookupBoth => return (Some(keysym), Some(s)),
                    _ => unreachable!{},
                }
            }
        };
    }
    fn x_event_to_event(&mut self, x_event: x::XEvent) -> Result<Event, ()> {

        unsafe {
            let x_display = self.x_display;
            let mut cookie = x::XGenericEventCookie::from(&x_event);
            if x::XGetEventData(x_display, &mut cookie) == x::True {
                if cookie.type_ == x::GenericEvent && cookie.extension == G_XI_OPCODE {
                    let e = self.xi_event_to_event(&cookie);
                    x::XFreeEventData(x_display, &mut cookie);
                    return e;
                }
            }
            x::XFreeEventData(x_display, &mut cookie); // Even if XGetEventData() failed.
        }

        match x_event.get_type() {
            KeyPress => {
                let x_event = unsafe { x_event.key };
                let window = self.x_window_to_window(x_event.window);
                let is_repeat = {
                    self.previous_x_key_release_time.get() == x_event.time
                 && self.previous_x_key_release_keycode.get() == x_event.keycode
                };
                let is_text = unsafe {
                    let f = x::XFilterEvent(&x_event as *const _ as *mut _, 0);
                    if f == x::False { true } else { false }
                };
                let (keysym, text) = match window.as_ref() {
                    Some(window) => {
                        window.os_window.set_net_wm_user_time(x_event.time);
                        if let Some(xic) = window.os_window.xic {
                            self.x_utf8_lookup_string(xic, &x_event)
                        } else {
                            (None, None)
                        }
                    },
                    None => (None, None),
                };
                let keysym = match keysym {
                    None => {
                        warn!{"Discarding event {:?} because of unknown keysym", x_event};
                        return Err(());
                    },
                    Some(x) => x,
                };
                let event = Event::KeyboardKeyPressed {
                    keyboard: self.dummy_keyboard(),
                    window,
                    os_scancode: x_event.keycode as _,
                    key: self.x_keysym_to_key(keysym),
                    is_repeat,
                    text: if is_text { text } else { None },
                };
                Ok(event)
            },
            KeyRelease => {
                let x_event = unsafe { x_event.key };
                self.previous_x_key_release_time.set(x_event.time);
                self.previous_x_key_release_keycode.set(x_event.keycode);
                let event = Event::KeyboardKeyReleased {
                    keyboard: self.dummy_keyboard(),
                    window: self.x_window_to_window(x_event.window),
                    os_scancode: x_event.keycode as _,
                    key: self.x_key_event_to_key(&x_event),
                };
                Ok(event)
            },
            ClientMessage => {
                let mut x_event = unsafe { x_event.client_message };
                let atoms = &self.atoms;
                if x_event.message_type != atoms.WM_PROTOCOLS {
                    return Err(());
                }
                if x_event.format != 32 {
                    return Err(());
                }
                if x_event.data.get_long(0) == atoms.WM_DELETE_WINDOW as _ {
                    if let Some(window) = self.x_window_to_window(x_event.window) {
                        return Ok(Event::WindowCloseRequested { window });
                    }
                }
                if x_event.data.get_long(0) == atoms._NET_WM_PING as _ {
                    x_event.window = x::XDefaultRootWindow(self.x_display);
                    x::XSendEvent(self.x_display, x_event.window, x::False, 
                        x::SubstructureNotifyMask | x::SubstructureRedirectMask,
                        &mut x_event as *mut _ as *mut _
                    );
                    return Err(()); // We handled it but we have no equivalent in our API.
                }
                warn!("Unhandled ClientMessage event: {:?}", x_event);
                Err(())
            },
            ButtonPress => {
                let x_event = unsafe { x_event.button };
                let position = Vec2 { x: x_event.x as _, y: x_event.y as _ };
                let abs_position = Vec2 { x: x_event.x_root as _, y: x_event.y_root as _ };
                let displacement = abs_position - self.previous_abs_mouse_position.get();
                let click = Click::Single;
                let window = self.x_window_to_window(x_event.window);
                let mouse = self.dummy_mouse();
                self.previous_abs_mouse_position.set(abs_position);
                if let Some(window) = window.as_ref() {
                    window.os_window.set_net_wm_user_time(x_event.time);
                }
                // http://xahlee.info/linux/linux_x11_mouse_button_number.html
                // On my R.AT 7, 10 is right scroll and 11 is left scroll.
                let scroll = match x_event.button {
                    4 => Some(Vec2::new(0,  1)),
                    5 => Some(Vec2::new(0, -1)),
                    6 => Some(Vec2::new(-1, 0)),
                    7 => Some(Vec2::new( 1, 0)),
                    _ => None,
                };
                let button = match x_event.button {
                    1 => Some(MouseButton::Left),
                    2 => Some(MouseButton::Middle),
                    3 => Some(MouseButton::Right),
                    4 => None,
                    5 => None,
                    6 => None,
                    7 => None,
                    8 => Some(MouseButton::Back),
                    9 => Some(MouseButton::Forward),
                    b @ _ => Some(MouseButton::Extra(b)),
                };
                if let Some(button) = button {
                    return Ok(Event::MouseButtonPressed {
                        mouse, window, position, abs_position, button, click, displacement,
                    });
                }
                if let Some(scroll) = scroll {
                    return Ok(Event::MouseScroll {
                        mouse, window, position, abs_position, scroll, displacement,
                    });
                }
                Err(())
            },
            ButtonRelease => {
                let x_event = unsafe { x_event.button };
                let position = Vec2 { x: x_event.x as _, y: x_event.y as _ };
                let abs_position = Vec2 { x: x_event.x_root as _, y: x_event.y_root as _ };
                let displacement = abs_position - self.previous_abs_mouse_position.get();
                let button = match x_event.button {
                    1 => MouseButton::Left,
                    2 => MouseButton::Middle,
                    3 => MouseButton::Right,
                    _ => return Err(()), // "scroll" buttons.
                };
                let window = self.x_window_to_window(x_event.window);
                let mouse = self.dummy_mouse();
                let event = Event::MouseButtonReleased {
                    mouse, window, position, abs_position, button, displacement,
                };
                self.previous_abs_mouse_position.set(abs_position);
                Ok(event)
            },
            MotionNotify => {
                let x_event = unsafe { x_event.motion };
                let position = Vec2 { x: x_event.x as _, y: x_event.y as _ };
                let abs_position = Vec2 { x: x_event.x_root as _, y: x_event.y_root as _ };
                let window = self.x_window_to_window(x_event.window);
                let displacement = abs_position - self.previous_abs_mouse_position.get();
                let mouse = self.dummy_mouse();
                if let Some(window) = window.as_ref() {
                    window.os_window.set_net_wm_user_time(x_event.time);
                }
                let event = Event::MouseMotion {
                    mouse, window, position, abs_position, displacement,
                };
                self.previous_abs_mouse_position.set(abs_position);
                Ok(event)
            }
            EnterNotify      => {
                let x_event = unsafe { x_event.crossing };
                let window = match self.x_window_to_window(x_event.window) {
                    None => return Err(()),
                    Some(w) => w,
                };
                let position = Vec2 { x: x_event.x as _, y: x_event.y as _ };
                let abs_position = Vec2 { x: x_event.x_root as _, y: x_event.y_root as _ };
                let mouse = self.dummy_mouse();
                let event = match x_event.mode {
                    x::NotifyNormal => Event::MouseEnter {
                        mouse, window, position, abs_position,
                    },
                    x::NotifyGrab => Event::MouseFocusGained {
                        mouse, window, position, abs_position,
                    },
                    _ => unreachable!{},
                };
                self.previous_abs_mouse_position.set(abs_position);
                Ok(event)
            },
            LeaveNotify      => {
                let x_event = unsafe { x_event.crossing };
                let window = match self.x_window_to_window(x_event.window) {
                    None => return Err(()),
                    Some(w) => w,
                };
                let position = Vec2 { x: x_event.x as _, y: x_event.y as _ };
                let abs_position = Vec2 { x: x_event.x_root as _, y: x_event.y_root as _ };
                let displacement = abs_position - self.previous_abs_mouse_position.get();
                let mouse = self.dummy_mouse();
                let event = match x_event.mode {
                    x::NotifyNormal => Event::MouseLeave {
                        mouse, window, position, abs_position, displacement,
                    },
                    x::NotifyUngrab => Event::MouseFocusLost {
                        mouse, window, position, abs_position, displacement,
                    },
                    _ => unreachable!{},
                };
                self.previous_abs_mouse_position.set(abs_position);
                Ok(event)
            },
            FocusIn          => {
                let x_event = unsafe { x_event.focus_change };
                let window = match self.x_window_to_window(x_event.window) {
                    None => return Err(()),
                    Some(w) => w,
                };
                if let Some(xic) = window.os_window.xic {
                    x::XSetICFocus(xic);
                }
                let keyboard = self.dummy_keyboard();
                let event = Event::KeyboardFocusGained {
                    keyboard, window,
                };
                Ok(event)
            },
            FocusOut         => {
                let x_event = unsafe { x_event.focus_change };
                let window = match self.x_window_to_window(x_event.window) {
                    None => return Err(()),
                    Some(w) => w,
                };
                if let Some(xic) = window.os_window.xic {
                    x::XUnsetICFocus(xic);
                }
                let keyboard = self.dummy_keyboard();
                let event = Event::KeyboardFocusLost {
                    keyboard, window,
                };
                Ok(event)
            },
            KeymapNotify     => Err(()),
            Expose           => {
                let x_event = unsafe { x_event.expose };
                let window = match self.x_window_to_window(x_event.window) {
                    None => return Err(()),
                    Some(w) => w,
                };
                Ok(Event::WindowContentDamaged { window })
            },
            GraphicsExpose   => {
                let x_event = unsafe { x_event.expose };
                let window = match self.x_window_to_window(x_event.window) {
                    None => return Err(()),
                    Some(w) => w,
                };
                Ok(Event::WindowContentDamaged { window })
            },
            NoExpose         => Err(()),
            CirculateRequest => Err(()), // FIXME: Shouldn't we handle these *Request events ?
            ConfigureRequest => Err(()),
            MapRequest       => Err(()),
            ResizeRequest    => Err(()),
            CirculateNotify  => Err(()),
            CreateNotify     => Err(()),
            DestroyNotify    => Err(()),
            GravityNotify    => {
                // Window moved because its parent's size changed.
                Err(())
            },
            ConfigureNotify  => {
                let x_event = unsafe { x_event.configure };
                let position = Vec2 { x: x_event.x as _, y: x_event.y as _ };
                let size = Extent2 { w: x_event.width as _, h: x_event.height as _ };
                let window = match self.x_window_to_window(x_event.window) {
                    None => return Err(()),
                    Some(w) => w,
                };
                Ok(Event::WindowMovedResized { window, position, size })
            }, 
            MapNotify        => Err(()),
            MappingNotify    => {
                // Keyboard/mouse was remapped.
                let mut x_event = unsafe { x_event.mapping };
                x::XRefreshKeyboardMapping(&mut x_event);
                Err(())
            },
            ReparentNotify   => Err(()),
            UnmapNotify      => Err(()),
            VisibilityNotify => Err(()),
            ColormapNotify   => Err(()),
            PropertyNotify   => Err(()),
            SelectionClear   => Err(()),
            SelectionNotify  => Err(()),
            SelectionRequest => Err(()),
            _ => {
                trace!("Unknown event type {}", x_event.get_type());
                Err(())
            }
        }
    }
}


impl OsWindow {

    pub(crate) fn set_net_wm_user_time(&self, time: x::Time) {
        let atom = self.shared_context.atoms._NET_WM_USER_TIME;
        if atom == 0 {
            return;
        }
        unsafe {
            x::XChangeProperty(self.shared_context.x_display, self.x_window,
                atom, x::XA_CARDINAL, 32, x::PropModeReplace,
                &time as *const _ as *const _, 1
            );
        }
    }


    pub fn show(&self) -> Result<(), Error> {
        unsafe {
            let x_display = self.shared_context.x_display;
            let x_window = self.x_window;
            x::XMapWindow(x_display, x_window);
            x::XSync(x_display, x::False);
            // Syncing, otherwise, it would be possible
            // to swap buffers before the window is shown, which would
            // have no effect.
            Ok(())
        }
    }
    pub fn hide(&self) -> Result<(), Error> {
        unsafe {
            let x_display = self.shared_context.x_display;
            let x_window = self.x_window;
            x::XUnmapWindow(x_display, x_window);
            x::XSync(x_display, x::False);
            Ok(())
        }
    }
    pub fn set_title(&self, title: &str) -> Result<(), Error> {
        unsafe {
            let x_display = self.shared_context.x_display;
            let atoms = &self.shared_context.atoms;
            let mut title_prop: x::XTextProperty = mem::uninitialized();
            let title_ptr = CString::new(title).unwrap_or_default();
            let mut title_ptr = title_ptr.as_bytes_with_nul().as_ptr() as *mut u8;
            let title_ptr = &mut title_ptr as *mut _;
            let status = x::Xutf8TextListToTextProperty(
                x_display, mem::transmute(title_ptr), 1, x::XUTF8StringStyle, &mut title_prop
            );
            if status == x::Success as i32 {
                x::XSetTextProperty(x_display, self.x_window, &mut title_prop, atoms._NET_WM_NAME);
                x::XFree(title_prop.value as *mut _);
            }
            x::XFlush(x_display);
            Ok(())
        }
    }
    pub fn set_icon(&self, icon: Icon) -> Result<(), Error> {
        let x_display = self.shared_context.x_display;
        let atoms = &self.shared_context.atoms;
        let x_window = self.x_window;

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
                x_display, x_window, atoms._NET_WM_ICON, x::XA_CARDINAL, 32, 
                x::PropModeReplace, prop.as_ptr() as _, prop.len() as _
            );
            x::XFlush(x_display);
        }
        Ok(())
    }
    pub fn clear_icon(&self) -> Result<(), Error> {
        let x_display = self.shared_context.x_display;
        let atoms = &self.shared_context.atoms;
        let x_window = self.x_window;

        unsafe {
            x::XDeleteProperty(x_display, x_window, atoms._NET_WM_ICON);
        }
        Ok(())
    }
    // NOTE: _NET_WM_WINDOW_TYPE might be useful too, but doesn't grant much control
    pub fn set_style(&self, style: &WindowStyle) -> Result<(), Error> {
        let wmhints_atom = self.shared_context.atoms._MOTIF_WM_HINTS;
        if wmhints_atom == 0 {
            return Err(Failed("Setting window style requires the _MOTIF_WM_HINTS atom!".to_owned()));
        }
        let flags = mwm::HINTS_FUNCTIONS | mwm::HINTS_DECORATIONS;
        let mut decorations = 0;
        let mut functions = 0;
        let &WindowStyle { title_bar, borders } = style;
        if let Some(title_bar) = title_bar {
            decorations |= mwm::DECOR_BORDER | mwm::DECOR_TITLE | mwm::DECOR_MENU;
            functions   |= mwm::FUNC_MOVE;
            if title_bar.minimize_button {
                decorations |= mwm::DECOR_MINIMIZE;
                functions   |= mwm::FUNC_MINIMIZE;
            }
            if title_bar.maximize_button {
                decorations |= mwm::DECOR_MAXIMIZE;
                functions   |= mwm::FUNC_MAXIMIZE;
            }
            if title_bar.maximize_button {
                functions   |= mwm::FUNC_CLOSE;
            }
        }
        let hints = mwm::WMHints {
            flags, decorations, functions, input_mode: 0, state: 0,
        };
        unsafe {
            x::XChangeProperty(self.shared_context.x_display, self.x_window,
                wmhints_atom, wmhints_atom, 32, x::PropModeReplace,
                &hints as *const _ as *const _, 5
            );
        }
        Ok(())
    }
    pub fn recenter(&self) -> Result<(), Error> {
        unimplemented!{}
    }
    pub fn set_opacity(&self, opacity: f32) -> Result<(), Error> {
        unimplemented!{}
    }
    fn get_geometry(&self) -> Rect<i32, u32> {
        unsafe {
            let x_display = self.shared_context.x_display;
            let x_window = self.x_window;
            let mut out_root: x::Window = 0;
            let (mut x, mut y): (c_int, c_int) = (0, 0);
            let (mut w, mut h, mut border, mut depth): (c_uint, c_uint, c_uint, c_uint) = (0, 0, 0, 0);
            x::XSync(x_display, x::False);
            let status = x::XGetGeometry(
                x_display, x_window, &mut out_root,
                &mut x, &mut y, &mut w, &mut h, &mut border, &mut depth
            );
            Rect {
                x: x as _,
                y: y as _,
                w: w as _,
                h: h as _,
            }
        }
    }
    pub fn query_screenspace_size(&self) -> Extent2<u32> {
        let Rect { w, h, .. } = self.get_geometry();
        Extent2 { w, h }
    }
    pub fn query_canvas_size(&self) -> Extent2<u32> {
        self.query_screenspace_size()
    }
    pub fn set_net_wm_window_type(&self, t: &[NetWMWindowType]) -> Result<(), Error> {
        let atoms = &self.shared_context.atoms;
        let mut value: Vec<_> = t.iter().map(|t| match t {
            Desktop       => atoms._NET_WM_WINDOW_TYPE_DESKTOP,
            Dock          => atoms._NET_WM_WINDOW_TYPE_DOCK,
            Toolbar       => atoms._NET_WM_WINDOW_TYPE_TOOLBAR,
            Menu          => atoms._NET_WM_WINDOW_TYPE_MENU,
            Utility       => atoms._NET_WM_WINDOW_TYPE_UTILITY,
            Splash        => atoms._NET_WM_WINDOW_TYPE_SPLASH,
            Dialog        => atoms._NET_WM_WINDOW_TYPE_DIALOG,
            DropdownMenu  => atoms._NET_WM_WINDOW_TYPE_DROPDOWN_MENU,
            PopupMenu     => atoms._NET_WM_WINDOW_TYPE_POPUP_MENU,
            Tooltip       => atoms._NET_WM_WINDOW_TYPE_TOOLTIP,
            Notification  => atoms._NET_WM_WINDOW_TYPE_NOTIFICATION,
            Combo         => atoms._NET_WM_WINDOW_TYPE_COMBO,
            DND           => atoms._NET_WM_WINDOW_TYPE_DND,
            Normal        => atoms._NET_WM_WINDOW_TYPE_NORMAL,
        }).collect();
        unsafe {
            x::XChangeProperty(
                self.shared_context.x_display, self.x_window,
                atoms._NET_WM_WINDOW_TYPE, x::XA_ATOM, 32,
                x::PropModeReplace, value.as_mut_ptr() as *mut _, value.len() as _
            );
        }
        Ok(())
    }
    pub fn maximize(&self) -> Result<(), Error> {
        self.set_net_wm_state(NetWMStateAction::Add,
            self.shared_context.atoms._NET_WM_STATE_MAXIMIZED_VERT,
            self.shared_context.atoms._NET_WM_STATE_MAXIMIZED_HORZ
        );
        Ok(())
    }
    pub fn unmaximize(&self) -> Result<(), Error> {
        self.set_net_wm_state(NetWMStateAction::Remove,
            self.shared_context.atoms._NET_WM_STATE_MAXIMIZED_VERT,
            self.shared_context.atoms._NET_WM_STATE_MAXIMIZED_HORZ
        );
        Ok(())
    }
    pub fn toggle_maximize(&self) -> Result<(), Error> {
        self.set_net_wm_state(NetWMStateAction::Toggle,
            self.shared_context.atoms._NET_WM_STATE_MAXIMIZED_VERT,
            self.shared_context.atoms._NET_WM_STATE_MAXIMIZED_HORZ
        );
        Ok(())
    }
    pub fn minimize(&self) -> Result<(), Error> {
        let status = unsafe {
            x::XIconifyWindow(
                self.shared_context.x_display, self.x_window,
                self.shared_context.screen_num
            )
        };
        if status != 0 {
            return Ok(());
        }
        Err(Failed(format!("XIconifyWindow() returned {}", status)))
    }
    pub fn restore(&self) -> Result<(), Error> {
        unsafe {
            x::XMapWindow(self.shared_context.x_display, self.x_window);
        }
        Ok(())
    }
    pub fn raise(&self) -> Result<(), Error> {
        unsafe {
            let x_display = self.shared_context.x_display;
            let x_window = self.x_window;
            x::XRaiseWindow(x_display, x_window);
            x::XSync(x_display, x::False);
            Ok(())
        }
    }
    
    fn set_net_wm_state(&self, action: NetWMStateAction, prop1: x::Atom, prop2: x::Atom) {
        unsafe {

            let x_display = self.shared_context.x_display;
            let screen = self.shared_context.screen_num;
            let atoms = &self.shared_context.atoms;
            let x_window = self.x_window;
            let root = x::XRootWindow(x_display, screen);

            let event_mask = x::SubstructureNotifyMask | x::SubstructureRedirectMask;

            let mut e = x::XClientMessageEvent {
                type_: x::ClientMessage,
                serial: 0,
                send_event: x::True,
                display: x_display,
                window: x_window,
                message_type: atoms._NET_WM_STATE,
                format: 32,
                data: Default::default(),
            };
            e.data.set_long(0, action as _);
            e.data.set_long(1, prop1 as _);
            e.data.set_long(2, prop2 as _);
            e.data.set_long(3, 1); // Normal window
            // ^ https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html#sourceindication

            x::XSendEvent(
                x_display, root, x::False,
                event_mask, &mut e as *mut _ as *mut x::XEvent
            );
            x::XSync(x_display, x::False);
        }
    }
    fn set_net_wm_state_fullscreen(&self, action: NetWMStateAction) -> Result<(), Error> {
        self.set_net_wm_state(action,
            self.shared_context.atoms._NET_WM_STATE_FULLSCREEN, 0
        );
        Ok(())
    }
    fn set_bypass_compositor(&self, doit: BypassCompositor) -> Result<(), Error> {
        let net_wm_bypass_compositor = self.shared_context.atoms._NET_WM_BYPASS_COMPOSITOR;
        if net_wm_bypass_compositor == 0 {
            return Err(Failed(format!("Can't set _NET_WM_BYPASS_COMPOSITOR to {:?}", doit)));
        }
        let mut value: c_ulong = doit as _;
        unsafe {
            x::XChangeProperty(
                self.shared_context.x_display, self.x_window,
                net_wm_bypass_compositor, x::XA_CARDINAL, 32,
                x::PropModeReplace, &mut value as *mut _ as *mut _, 1
            );
        }
        Ok(())
    }
    pub fn toggle_fullscreen(&self) -> Result<(), Error> {
        // XXX assuming we can't know
        let _ = self.set_bypass_compositor(BypassCompositor::NoPreference);
        self.set_net_wm_state_fullscreen(NetWMStateAction::Toggle)
    }
    pub fn enter_fullscreen(&self) -> Result<(), Error> {
        let _ = self.set_bypass_compositor(BypassCompositor::Yes);
        self.set_net_wm_state_fullscreen(NetWMStateAction::Add)
    }
    pub fn leave_fullscreen(&self) -> Result<(), Error> {
        let _ = self.set_bypass_compositor(BypassCompositor::NoPreference);
        self.set_net_wm_state_fullscreen(NetWMStateAction::Remove)
    }
    pub fn demand_attention(&self) -> Result<(), Error> {
        self.set_net_wm_state(
            NetWMStateAction::Add,
            self.shared_context.atoms._NET_WM_STATE_DEMANDS_ATTENTION, 0
        );
        Ok(())
    }

    pub fn set_minimum_size(&self, size: Extent2<u32>) -> Result<(), Error> {
        let x_display = self.shared_context.x_display;
        let x_window = self.x_window;
        unsafe {
            let mut hints = x::XAllocSizeHints();
            (*hints).flags = x::PMinSize;
            (*hints).min_width = size.w as _;
            (*hints).min_height = size.h as _;
            x::XSetWMNormalHints(x_display, x_window, hints);
            x::XFree(hints as _);
        }
        Ok(())
    }
    pub fn set_maximum_size(&self, size: Extent2<u32>) -> Result<(), Error> {
        let x_display = self.shared_context.x_display;
        let x_window = self.x_window;
        unsafe {
            let mut hints = x::XAllocSizeHints();
            (*hints).flags = x::PMaxSize;
            (*hints).max_width = size.w as _;
            (*hints).max_height = size.h as _;
            x::XSetWMNormalHints(x_display, x_window, hints);
            x::XFree(hints as _);
        }
        Ok(())
    }
    // WISH: Maybe use XTranslateCoordinates() instead ?
    pub fn position(&self) -> Result<Vec2<i32>, Error> {
        let Rect { x, y, .. } = self.get_geometry();
        Ok(Vec2 { x, y })
    }
    pub fn set_position(&self, pos: Vec2<i32>) -> Result<(), Error> {
        unsafe {
            let x_display = self.shared_context.x_display;
            let x_window = self.x_window;
            x::XMoveWindow(x_display, x_window, pos.x, pos.y);
            x::XSync(x_display, x::False);
            Ok(())
        }
    }
    pub fn resize(&self, size: Extent2<u32>) -> Result<(), Error> {
        unsafe {
            let x_display = self.shared_context.x_display;
            let x_window = self.x_window;
            x::XResizeWindow(x_display, x_window, size.w, size.h);
            x::XSync(x_display, x::False);
            Ok(())
        }
    }
    pub fn set_position_and_resize(&self, r: Rect<i32, u32>) -> Result<(), Error> {
        unsafe {
            let x_display = self.shared_context.x_display;
            let x_window = self.x_window;
            x::XMoveResizeWindow(x_display, x_window, r.x, r.y, r.w, r.h);
            x::XSync(x_display, x::False);
            Ok(())
        }
    }

    pub fn hide_cursor(&self) -> Result<(), Error> {
        self.shows_cursor.set(false);
        unsafe {
            x::XDefineCursor(self.shared_context.x_display, self.x_window, self.shared_context.invisible_x_cursor);
        }
        Ok(())
    }
    pub fn show_cursor(&self) -> Result<(), Error> {
        self.shows_cursor.set(true);
        let user_cursor = self.user_cursor.borrow();
        let user_cursor = match user_cursor.as_ref() {
            None => return Ok(()),
            Some(x) => x,
        };
        unsafe {
            x::XDefineCursor(self.shared_context.x_display, self.x_window, user_cursor.0.x_cursor);
        }
        Ok(())
    }
    pub fn set_cursor(&self, cursor: Rc<Cursor>) -> Result<(), Error> {
        *self.user_cursor.borrow_mut() = Some(cursor);
        if self.shows_cursor.get() {
            return self.show_cursor();
        }
        Ok(())
    }

    pub fn set_cursor_position(&self, pos: Vec2<u32>) -> Result<(), Error> {
        unsafe {
            x::XWarpPointer(
                self.shared_context.x_display, 0, self.x_window,
                0, 0, 0, 0, pos.x as _, pos.y as _
            );
        }
        Ok(())
    }
    pub fn query_cursor_position(&self) -> Result<Vec2<u32>, Error> {
        unsafe {
            let mut root: x::Window = 0;
            let mut child: x::Window = 0;
            let mut root_x: c_int = 0;
            let mut root_y: c_int = 0;
            let mut x: c_int = 0;
            let mut y: c_int = 0;
            let mut mask: c_uint = 0;
            let is_on_same_screen = x::XQueryPointer(
                self.shared_context.x_display, self.x_window,
                &mut root, &mut child, &mut root_x, &mut root_y,
                &mut x, &mut y, &mut mask
            );
            Ok(Vec2::new(x as _, y as _))
        }
    }

    pub fn make_gl_context_current(&self, gl_context: Option<&OsGLContext>) {
        let glx_context = match gl_context {
            None => ptr::null_mut(),
            Some(c) => c.glx_context,
        };
        let x_display = self.shared_context.x_display;
        unsafe {
            match self.glx_window {
                Some(w) => glXMakeContextCurrent(
                    x_display, w, w, glx_context
                ),
                None => glXMakeCurrent(
                    x_display, self.x_window, glx_context
                ),
            };
        }
    }
    pub fn gl_swap_buffers(&self) {
        let x_display = self.shared_context.x_display;
        unsafe {
            glXSwapBuffers(x_display, match self.glx_window {
                Some(w) => w,
                None => self.x_window,
            });
        }
    }
    pub fn set_gl_swap_interval(&mut self, interval: GLSwapInterval) -> Result<(), Error> {
        let glx = match self.shared_context.glx.as_ref() {
            None => return Err(Failed("GLX is not supported!".to_owned())),
            Some(glx) => glx,
        };
        let interval: c_int = match interval {
            GLSwapInterval::LimitFps(_) => unreachable!{}, // Already implemented by wrapper
            GLSwapInterval::VSync => 1,
            GLSwapInterval::Immediate => 0,
            GLSwapInterval::LateSwapTearing => {
                if !glx.ext.GLX_EXT_swap_control_tear {
                    return Err(Failed("Late swap tearing requires GLX_EXT_swap_control_tear".to_owned()));
                }
                -1
            },
            GLSwapInterval::Interval(i) => {
                if i < 0 && !glx.ext.GLX_EXT_swap_control_tear {
                    return Err(Failed("Late swap tearing requires GLX_EXT_swap_control_tear".to_owned()));
                }
                i
            },
        };

        if glx.ext.GLX_EXT_swap_control && self.glx_window.is_some() {
            let ssi = glx.ext.glXSwapIntervalEXT.unwrap();
            unsafe {
                ssi(self.shared_context.x_display, self.glx_window.unwrap(), interval);
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
            Err(Failed("No GLX extension that could set the swap interval is present".to_owned()))
        }
    }
}

pub type OsGLProc = unsafe extern "C" fn();

impl OsGLContext {
    // NOTE: glXGetProcAddressARB doesn't need a bound context, unlike in WGL.
    pub(super) unsafe fn get_proc_address(&self, name: *const c_char) -> Option<OsGLProc> {
        #[cfg(not(target_os = "linux"))]
        unimplemented!("We don't know how the situation is in OSes other than Linux! This could require moving to x11-dl.");
        #[cfg(target_os = "linux")]
        glXGetProcAddressARB(name as *const _)
    }
}

impl OsHid {
    pub fn is_connected(&self) -> bool {
        unimplemented!{}
    }
}

