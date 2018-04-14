#![allow(dead_code)]

pub use window::NetWMWindowType;

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

