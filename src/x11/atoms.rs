use std::ffi::CString;
use super::x11::xlib as x;

/// Generate this module's `PreparedAtoms` struct, where all atoms are retrieved
/// once when opening a display.
macro_rules! atoms {
    ($($atom:ident)+) => {
        #[allow(non_snake_case)]
        #[derive(Debug, Hash, PartialEq, Eq)]
        pub struct PreloadedAtoms {
            $(pub $atom: x::Atom,)+
        }
        #[allow(non_snake_case)]
        impl PreloadedAtoms {
            pub fn load(x_display: *mut x::Display) -> Self {
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


