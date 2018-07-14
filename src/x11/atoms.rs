use std::os::raw::c_char;
use std::ffi::CStr;
use super::x11::xlib as x;
use super::xlib_error;
use error::{Result, failed};

/// Generate this module's `PreparedAtoms` struct, where all atoms are retrieved
/// once after opening a display.
macro_rules! atoms {
    ($($atom:ident => $name:expr,)+) => {
        #[repr(C)] // Passed directly as array of Atoms to XInternAtoms()
        #[allow(non_snake_case)]
        #[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
        pub struct PreloadedAtoms {
            $($atom: x::Atom,)+ // Make sure the atom fields come first!
            interesting_xi2_props: Vec<x::Atom>,
        }
        const ATOM_NAMES: &'static [*const c_char] = &[
            $($name as *const _ as *const c_char,)+
        ];
        #[allow(non_snake_case)]
        impl PreloadedAtoms {
            fn err_atom_not_present(name: &[u8]) -> Result<x::Atom> {
                failed(format!("`{}` atom is not present", CStr::from_bytes_with_nul(name).unwrap().to_string_lossy()))
            }
            fn err_atom_not_registered(name: &[u8]) -> Result<x::Atom> {
                failed(format!("`{}` atom was not registered", CStr::from_bytes_with_nul(name).unwrap().to_string_lossy()))
            }
            $(
                #[allow(non_snake_case, dead_code)]
                pub fn $atom(&self) -> Result<x::Atom> {
                    match self.$atom {
                        0 => Self::err_atom_not_present($name),
                        atom => Ok(atom),
                    }
                }
            )+
            pub fn from_name_with_nul(&self, name: &[u8]) -> Result<x::Atom> {
                assert_eq!(&0, name.last().unwrap());
                match name {
                    $($name => self.$atom(),)+
                    name => Self::err_atom_not_registered(name),
                }
            }

            pub fn load(x_display: *mut x::Display) -> Result<Self> {
                $(assert_eq!(&0, $name.last().unwrap());)+
                let mut atoms = Self::default();
                unsafe {
                    xlib_error::sync_catch(x_display, || {
                        let only_if_exists = x::True;
                        let _were_all_of_these_atoms_present = x::XInternAtoms(
                            x_display, 
                            ATOM_NAMES.as_ptr() as *mut _, 
                            ATOM_NAMES.len() as _,
                            only_if_exists,
                            &mut atoms as *mut _ as *mut x::Atom
                        );
                    })?;
                }

                atoms.interesting_xi2_props = [
                    atoms.Device_Node(),
                    atoms.Wacom_Tablet_Area(),
                    atoms.Wacom_Serial_IDs(),
                    atoms.Wacom_Tool_Type(),
                ].iter().filter_map(|x| x.as_ref().ok().map(|x| *x)).collect();

                Ok(atoms)
            }
            pub fn interesting_xi2_props(&self) -> &[x::Atom] {
                &self.interesting_xi2_props
            }
            pub fn is_interesting_xi2_prop(&self, prop: x::Atom) -> bool {
                self.interesting_xi2_props.contains(&prop)
            }
        }
    }
}

// NOTE: It looks annoying to have to write each atom name twice,
// but here are reasons for it :
// - There exist atoms which name is not a valid Rust identifier.
// - We avoid stringify!() which would imply one CString allocation per atom.
atoms!{
    // Some base atoms
    UTF8_STRING => b"UTF8_STRING\0",
    PRIMARY     => b"PRIMARY\0",
    SECONDARY   => b"SECONDARY\0",
    CLIPBOARD   => b"CLIPBOARD\0",

    // One mindlessly grabbed from SDL2
    XKLAVIER_STATE => b"XKLAVIER_STATE\0",

    // Some ICCCM atoms
    WM_PROTOCOLS     => b"WM_PROTOCOLS\0",
    WM_DELETE_WINDOW => b"WM_DELETE_WINDOW\0",
    WM_TAKE_FOCUS    => b"WM_TAKE_FOCUS\0",
    WM_STATE         => b"WM_STATE\0",
    WM_CHANGE_STATE  => b"WM_CHANGE_STATE\0",

    // Motif
    _MOTIF_WM_HINTS => b"_MOTIF_WM_HINTS\0",

    // EWMH atoms
    _NET_SUPPORTED           => b"_NET_SUPPORTED\0",
    _NET_CLIENT_LIST         => b"_NET_CLIENT_LIST\0",
    _NET_NUMBER_OF_DESKTOPS  => b"_NET_NUMBER_OF_DESKTOPS\0",
    _NET_DESKTOP_GEOMETRY    => b"_NET_DESKTOP_GEOMETRY\0",
    _NET_DESKTOP_VIEWPORT    => b"_NET_DESKTOP_VIEWPORT\0",
    _NET_CURRENT_DESKTOP     => b"_NET_CURRENT_DESKTOP\0",
    _NET_DESKTOP_NAMES       => b"_NET_DESKTOP_NAMES\0",
    _NET_ACTIVE_WINDOW       => b"_NET_ACTIVE_WINDOW\0",
    _NET_WORKAREA            => b"_NET_WORKAREA\0",
    _NET_SUPPORTING_WM_CHECK => b"_NET_SUPPORTING_WM_CHECK\0",
    _NET_VIRTUAL_ROOTS       => b"_NET_VIRTUAL_ROOTS\0",
    _NET_DESKTOP_LAYOUT      => b"_NET_DESKTOP_LAYOUT\0",
    _NET_SHOWING_DESKTOP     => b"_NET_SHOWING_DESKTOP\0",

    _NET_CLOSE_WINDOW          => b"_NET_CLOSE_WINDOW\0",
    _NET_MOVERESIZE_WINDOW     => b"_NET_MOVERESIZE_WINDOW\0",
    _NET_WM_MOVERESIZE         => b"_NET_WM_MOVERESIZE\0",
    _NET_RESTACK_WINDOW        => b"_NET_RESTACK_WINDOW\0",
    _NET_REQUEST_FRAME_EXTENTS => b"_NET_REQUEST_FRAME_EXTENTS\0",
 
    _NET_WM_NAME               => b"_NET_WM_NAME\0",
    _NET_WM_VISIBLE_NAME       => b"_NET_WM_VISIBLE_NAME\0",
    _NET_WM_ICON_NAME          => b"_NET_WM_ICON_NAME\0",
    _NET_WM_VISIBLE_ICON_NAME  => b"_NET_WM_VISIBLE_ICON_NAME\0",
    _NET_WM_DESKTOP            => b"_NET_WM_DESKTOP\0",
 
    _NET_WM_WINDOW_TYPE               => b"_NET_WM_WINDOW_TYPE\0",
    _NET_WM_WINDOW_TYPE_DESKTOP       => b"_NET_WM_WINDOW_TYPE_DESKTOP\0",
    _NET_WM_WINDOW_TYPE_DOCK          => b"_NET_WM_WINDOW_TYPE_DOCK\0",
    _NET_WM_WINDOW_TYPE_TOOLBAR       => b"_NET_WM_WINDOW_TYPE_TOOLBAR\0",
    _NET_WM_WINDOW_TYPE_MENU          => b"_NET_WM_WINDOW_TYPE_MENU\0",
    _NET_WM_WINDOW_TYPE_UTILITY       => b"_NET_WM_WINDOW_TYPE_UTILITY\0",
    _NET_WM_WINDOW_TYPE_SPLASH        => b"_NET_WM_WINDOW_TYPE_SPLASH\0",
    _NET_WM_WINDOW_TYPE_DIALOG        => b"_NET_WM_WINDOW_TYPE_DIALOG\0",
    _NET_WM_WINDOW_TYPE_DROPDOWN_MENU => b"_NET_WM_WINDOW_TYPE_DROPDOWN_MENU\0",
    _NET_WM_WINDOW_TYPE_POPUP_MENU    => b"_NET_WM_WINDOW_TYPE_POPUP_MENU\0",
    _NET_WM_WINDOW_TYPE_TOOLTIP       => b"_NET_WM_WINDOW_TYPE_TOOLTIP\0",
    _NET_WM_WINDOW_TYPE_NOTIFICATION  => b"_NET_WM_WINDOW_TYPE_NOTIFICATION\0",
    _NET_WM_WINDOW_TYPE_COMBO         => b"_NET_WM_WINDOW_TYPE_COMBO\0",
    _NET_WM_WINDOW_TYPE_DND           => b"_NET_WM_WINDOW_TYPE_DND\0",
    _NET_WM_WINDOW_TYPE_NORMAL        => b"_NET_WM_WINDOW_TYPE_NORMAL\0",
 
    _NET_WM_STATE                   => b"_NET_WM_STATE\0",
    _NET_WM_STATE_MODAL             => b"_NET_WM_STATE_MODAL\0",
    _NET_WM_STATE_STICKY            => b"_NET_WM_STATE_STICKY\0",
    _NET_WM_STATE_MAXIMIZED_VERT    => b"_NET_WM_STATE_MAXIMIZED_VERT\0",
    _NET_WM_STATE_MAXIMIZED_HORZ    => b"_NET_WM_STATE_MAXIMIZED_HORZ\0",
    _NET_WM_STATE_SHADED            => b"_NET_WM_STATE_SHADED\0",
    _NET_WM_STATE_SKIP_TASKBAR      => b"_NET_WM_STATE_SKIP_TASKBAR\0",
    _NET_WM_STATE_SKIP_PAGER        => b"_NET_WM_STATE_SKIP_PAGER\0",
    _NET_WM_STATE_HIDDEN            => b"_NET_WM_STATE_HIDDEN\0",
    _NET_WM_STATE_FULLSCREEN        => b"_NET_WM_STATE_FULLSCREEN\0",
    _NET_WM_STATE_ABOVE             => b"_NET_WM_STATE_ABOVE\0",
    _NET_WM_STATE_BELOW             => b"_NET_WM_STATE_BELOW\0",
    _NET_WM_STATE_DEMANDS_ATTENTION => b"_NET_WM_STATE_DEMANDS_ATTENTION\0",
    _NET_WM_STATE_FOCUSED           => b"_NET_WM_STATE_FOCUSED\0",
 
    _NET_WM_ALLOWED_ACTIONS         => b"_NET_WM_ALLOWED_ACTIONS\0",
    _NET_WM_ACTION_MOVE             => b"_NET_WM_ACTION_MOVE\0",
    _NET_WM_ACTION_RESIZE           => b"_NET_WM_ACTION_RESIZE\0",
    _NET_WM_ACTION_MINIMIZE         => b"_NET_WM_ACTION_MINIMIZE\0",
    _NET_WM_ACTION_SHADE            => b"_NET_WM_ACTION_SHADE\0",
    _NET_WM_ACTION_STICK            => b"_NET_WM_ACTION_STICK\0",
    _NET_WM_ACTION_MAXIMIZE_HORZ    => b"_NET_WM_ACTION_MAXIMIZE_HORZ\0",
    _NET_WM_ACTION_MAXIMIZE_VERT    => b"_NET_WM_ACTION_MAXIMIZE_VERT\0",
    _NET_WM_ACTION_FULLSCREEN       => b"_NET_WM_ACTION_FULLSCREEN\0",
    _NET_WM_ACTION_CHANGE_DESKTOP   => b"_NET_WM_ACTION_CHANGE_DESKTOP\0",
    _NET_WM_ACTION_CLOSE            => b"_NET_WM_ACTION_CLOSE\0",
    _NET_WM_ACTION_ABOVE            => b"_NET_WM_ACTION_ABOVE\0",
    _NET_WM_ACTION_BELOW            => b"_NET_WM_ACTION_BELOW\0",
 
    _NET_WM_STRUT                   => b"_NET_WM_STRUT\0",
    _NET_WM_STRUT_PARTIAL           => b"_NET_WM_STRUT_PARTIAL\0",
    _NET_WM_ICON_GEOMETRY           => b"_NET_WM_ICON_GEOMETRY\0",
    // This is an array of 32bit packed CARDINAL ARGB with high byte being A, low byte being B. The first two cardinals are width, height. Data is in rows, left to right and top to bottom.
    _NET_WM_ICON                    => b"_NET_WM_ICON\0",

    _NET_WM_PID                 => b"_NET_WM_PID\0",
    _NET_WM_HANDLED_ICONS       => b"_NET_WM_HANDLED_ICONS\0",
    _NET_WM_USER_TIME           => b"_NET_WM_USER_TIME\0",
    _NET_WM_USER_TIME_WINDOW    => b"_NET_WM_USER_TIME_WINDOW\0",
    _NET_FRAME_EXTENTS          => b"_NET_FRAME_EXTENTS\0",
    _NET_WM_OPAQUE_REGION       => b"_NET_WM_OPAQUE_REGION\0",
    _NET_WM_BYPASS_COMPOSITOR   => b"_NET_WM_BYPASS_COMPOSITOR\0",
 
    _NET_WM_PING                => b"_NET_WM_PING\0",
    _NET_WM_SYNC_REQUEST        => b"_NET_WM_SYNC_REQUEST\0",
    _NET_WM_FULLSCREEN_MONITORS => b"_NET_WM_FULLSCREEN_MONITORS\0",
    _NET_WM_FULL_PLACEMENT      => b"_NET_WM_FULL_PLACEMENT\0",
    _NET_WM_WINDOW_OPACITY      => b"_NET_WM_WINDOW_OPACITY\0",

    // X Drag'n Drop atoms
    // Also don't forget to check:
    // https://www.freedesktop.org/wiki/Draganddropwarts/
    XdndAware             => b"XdndAware\0",
    XdndEnter             => b"XdndEnter\0",
    XdndPosition          => b"XdndPosition\0",
    XdndLeave             => b"XdndLeave\0",
    XdndStatus            => b"XdndStatus\0",
    XdndTypeList          => b"XdndTypeList\0",
    XdndDrop              => b"XdndDrop\0",
    XdndFinished          => b"XdndFinished\0",
    XdndSelection         => b"XdndSelection\0",
    XdndActionCopy        => b"XdndActionCopy\0",
    XdndActionMove        => b"XdndActionMove\0",
    XdndActionLink        => b"XdndActionLink\0",
    XdndActionAsk         => b"XdndActionAsk\0",
    XdndActionPrivate     => b"XdndActionPrivate\0",
    XdndActionList        => b"XdndActionList\0",
    XdndActionDescription => b"XdndActionDescription\0",
    XdndProxy             => b"XdndProxy\0",
 
    _MOTIF_DRAG_AND_DROP_MESSAGE => b"_MOTIF_DRAG_AND_DROP_MESSAGE\0",
    _MOTIF_DRAG_INITIATOR_INFO   => b"_MOTIF_DRAG_INITIATOR_INFO\0",
    _MOTIF_DRAG_RECEIVER_INFO    => b"_MOTIF_DRAG_RECEIVER_INFO\0",
    _MOTIF_DRAG_WINDOW           => b"_MOTIF_DRAG_WINDOW\0",
    _MOTIF_DRAG_TARGETS          => b"_MOTIF_DRAG_TARGETS\0",

    XWacomStylus  => b"XWacomStylus\0",
    XWacomCursor  => b"XWacomCursor\0",
    XWacomEraser  => b"XWacomEraser\0",
    XTabletStylus => b"XTabletStylus\0",
    XTabletEraser => b"XTabletEraser\0",

    // XInput2 relative axis labels
    Rel_X              => b"Rel X\0",
    Rel_Y              => b"Rel Y\0",
    Rel_Horiz_Scroll   => b"Rel Horiz Scroll\0",
    Rel_Vert_Scroll    => b"Rel Vert Scroll\0",
    // XInput2 absolute axis labels
    Abs_MT_Touch_Major => b"Abs MT Touch Major\0",
    Abs_MT_Pressure    => b"Abs MT Pressure\0",
    Abs_X              => b"Abs X\0",
    Abs_Y              => b"Abs Y\0",
    Abs_Pressure       => b"Abs Pressure\0",
    Abs_Tilt_X         => b"Abs Tilt X\0",
    Abs_Tilt_Y         => b"Abs Tilt Y\0",
    Abs_Wheel          => b"Abs Wheel\0",
    // XInput2 mouse buttons labels
    Button_Left              => b"Button Left\0",
    Button_Middle            => b"Button Middle\0",
    Button_Right             => b"Button Right\0",
    Button_Side              => b"Button Side\0",
    Button_Extra             => b"Button Extra\0",
    Button_Forward           => b"Button Forward\0",
    Button_Back              => b"Button Back\0",
    Button_Task              => b"Button Task\0",
    Button_Unknown           => b"Button Unknown\0",
    Button_Wheel_Up          => b"Button Wheel Up\0",
    Button_Wheel_Down        => b"Button Wheel Down\0",
    Button_Horiz_Wheel_Left  => b"Button Horiz Wheel Left\0",
    Button_Horiz_Wheel_Right => b"Button Horiz Wheel Right\0",
    // XInput2 properties.
    // If you add any, make sure to update the `interesting_xi2_props` initialization!
    Device_Node       => b"Device Node\0", // e.g "/dev/input/event14"
    Wacom_Tablet_Area => b"Wacom Tablet Area\0", // e.g: 0, 0, 21648, 13700
    Wacom_Serial_IDs  => b"Wacom Serial IDs\0", // e.g: 216, 0, 2, 0, 0
    Wacom_Tool_Type   => b"Wacom Tool Type\0", // One of the atoms right below
        STYLUS            => b"STYLUS\0",
        ERASER            => b"ERASER\0",
        PAD               => b"PAD\0",
        TOUCH             => b"TOUCH\0",
}

