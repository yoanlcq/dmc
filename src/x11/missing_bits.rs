// TODO: Send a PR to x11-rs.
// Missing items for X11
pub mod x {
    pub const GLX_CONTEXT_ES_PROFILE_BIT_EXT             : i32 = 0x00000004;
    pub const GLX_CONTEXT_ES2_PROFILE_BIT_EXT            : i32 = 0x00000004;
    pub const GLX_CONTEXT_ROBUST_ACCESS_BIT_ARB          : i32 = 0x00000004;
    pub const GLX_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB: i32 = 0x8256;
    pub const GLX_NO_RESET_NOTIFICATION_ARB              : i32 = 0x8261;
    pub const GLX_LOSE_CONTEXT_ON_RESET_ARB              : i32 = 0x8252;
}

// TODO: Send a PR to x11-rs.
// Missing items for XInput
#[allow(non_upper_case_globals)]
pub mod xi {
    pub const NoSuchExtension: i32 = 1;
}

pub mod xrender {
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

// Motif Window Manager
pub mod mwm {
    use ::std::os::raw::{c_long, c_ulong};
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

