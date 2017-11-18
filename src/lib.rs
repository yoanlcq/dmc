//! DMC - Display and Multimedia Context
//! 
//! This is an attempt at an SDL2 rewrite in Rust. The end goal is to get
//! rid of the dependency on SDL2's DLL for Rust apps.

#![doc(html_root_url = "https://docs.rs/dmc/0.1.0")]
//#![feature(test)]
//#![warn(missing_docs)]
#![doc(test(attr(deny(warnings))))]
#![cfg_attr(feature="cargo-clippy", allow(doc_markdown))]

#![allow(warnings)] // FIXME: Remove before release

extern crate num_traits;
extern crate vek;
#[macro_use]
extern crate log;

// Nontrivial modules go first
pub mod context;
pub use context::Context;
pub mod window;
pub use window::Window;
pub mod hid;
pub use hid::*;
pub mod event;
pub use event::{Event, Click};

pub mod semver;
pub use semver::Semver;
pub mod timeout;
pub use timeout::Timeout;
pub mod battery;
pub use battery::{BatteryState, BatteryStatus};

#[macro_use]
mod option_alternative;
pub mod decision;
pub use decision::Decision;
pub use decision::Decision::*;
pub mod knowledge;
pub use knowledge::Knowledge;
pub use knowledge::Knowledge::*;


#[cfg(dmc_display_backend="x11")]
#[path="backends/x11.rs"]
mod backend;

#[cfg(not(dmc_display_backend="x11"))]
#[path="backends/unimplemented.rs"]
mod backend;


