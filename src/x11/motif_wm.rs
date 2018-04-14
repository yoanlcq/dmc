#![allow(dead_code)]
use ::std::os::raw::{c_long, c_ulong};

pub mod flags {
    use super::*;
    pub const FUNCTIONS   : c_ulong = 1 << 0;
    pub const DECORATIONS : c_ulong = 1 << 1;
}
pub mod decorations {
    use super::*;
    pub const ALL         : c_ulong = 1 << 0;
    pub const BORDER      : c_ulong = 1 << 1;
    pub const RESIZE      : c_ulong = 1 << 2;
    pub const TITLE       : c_ulong = 1 << 3;
    pub const MENU        : c_ulong = 1 << 4;
    pub const MINIMIZE    : c_ulong = 1 << 5;
    pub const MAXIMIZE    : c_ulong = 1 << 6;
}
pub mod functions {
    use super::*;
    pub const ALL          : c_ulong = 1 << 0;
    pub const RESIZE       : c_ulong = 1 << 1;
    pub const MOVE         : c_ulong = 1 << 2;
    pub const MINIMIZE     : c_ulong = 1 << 3;
    pub const MAXIMIZE     : c_ulong = 1 << 4;
    pub const CLOSE        : c_ulong = 1 << 5;
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(C)]
pub struct Hints {
    pub flags      : c_ulong,
    pub functions  : c_ulong,
    pub decorations: c_ulong,
    pub input_mode : c_long,
    pub state      : c_ulong,
}

impl From<[c_ulong; 5]> for Hints {
    fn from(data: [c_ulong; 5]) -> Self {
        Self { 
            flags       : data[0], 
            functions   : data[1], 
            decorations : data[2], 
            input_mode  : data[3] as _, 
            state       : data[4],
        }
    }
}

impl Hints {
    pub fn into_array(self) -> [c_ulong; 5] {
        let Self { flags, functions, decorations, input_mode, state } = self;
        [flags, functions, decorations, input_mode as _, state]
    }
}
