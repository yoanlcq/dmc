//! DMC - DirectMedia Crate
//! 
//! This is an attempt at an SDL2 rewrite in Rust. The end goal is to get
//! rid of the dependency on SDL2's DLL for Rust apps.
//!
//! TODO be able to provide WM_QUERYENDSESSION and WM_ENDSESSION events (Win32)- See [Shutting Down](https://msdn.microsoft.com/en-us/library/windows/desktop/aa376881(v=vs.85).aspx)

#![doc(html_root_url = "https://docs.rs/dmc/0.1.0")]
//#![feature(test)]
//#![warn(missing_docs)]
#![doc(test(attr(deny(warnings))))]
#![cfg_attr(feature="cargo-clippy", allow(doc_markdown))]

extern crate num_traits;
extern crate uuid;
extern crate vek;
#[macro_use]
extern crate log;
extern crate libc;

pub use vek::vec::repr_c::{
    Vec2, Vec3, Extent2, Rgba, Rgb,
};
pub type Rgba32 = Rgba<u8>;

pub mod semver;
pub use semver::Semver;
pub mod display;
pub use display::Display;
pub mod hid;
pub use hid::{GameInputDevice, Dpad, Minmax, SignedAxis, UnsignedAxis};
pub mod event;
pub use event::{EventQueue, Clipboard, TextInput};
pub mod battery;
pub use battery::{BatteryState, BatteryStatus};
pub mod timeout;
pub use timeout::Timeout;
pub mod option_alternatives;
pub use option_alternatives::Decision;
pub use option_alternatives::Knowledge;
pub use option_alternatives::Decision::*;
pub use option_alternatives::Knowledge::*;
