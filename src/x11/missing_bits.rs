#![allow(dead_code)]

// TODO: Send a PR to x11-rs.
// Missing items for X11
pub mod glx {
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

// TODO: Send a PR to x11-rs.
// Missing items for XInput
#[allow(non_upper_case_globals)]
pub mod xutil {
    pub const XNoMemory: i32 = -1;
    pub const XLocaleNotSupported: i32 = -2;
    pub const XConverterNotFound: i32 = -3;
}

#[allow(non_upper_case_globals)]
pub mod wm_state {
    pub const WithdrawnState: i32 = 0;
    pub const NormalState: i32 = 1;
    pub const IconicState: i32 = 3;
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

