//! DMC - Display and Multimedia Context
//! 
//! This is an attempt at an SDL2 rewrite in Rust. The end goal is to get
//! rid of the dependency on SDL2's shared library for Rust applications.

#![doc(html_root_url = "https://docs.rs/dmc/0.2.0")]
#![feature(optin_builtin_traits)] // !Send, !Sync for Context
#![warn(missing_docs)]

#[allow(unused_imports)]
#[macro_use]
extern crate log;
extern crate vek;


pub mod error;
pub use error::{ErrorKind, Error};
pub mod context;
pub use context::Context;
pub mod window;
pub use window::{Window, WindowSettings, WindowMode};
pub mod gl;
/*
pub mod cursor;
pub mod hid;
pub use hid::*;
pub mod event;
pub mod battery;
*/

mod version_cmp;

macro_rules! os_mod {
    ($os:ident) => {
        mod os {
            pub mod $os;
            pub use self::$os::{
                OsContext, OsWindow,
                OsGLPixelFormat, OsGLContext, OsGLProc,
            };
        }

        /// Raw OpenGL function type, with the appropriate calling convention for this platform.
        pub type OsGLProc = os::OsGLProc;
    };
}

#[cfg(any(target_os="linux", target_os="freebsd", target_os="dragonfly", target_os="openbsd", target_os="netbsd"))]
mod x11;

#[cfg(target_os="linux")]
os_mod!{linux}

#[cfg(any(target_os="freebsd", target_os="dragonfly", target_os="openbsd", target_os="netbsd"))]
os_mod!{bsd}

#[cfg(target_os="windows")]
os_mod!{windows}

#[cfg(target_os="winrt")]
os_mod!{winrt}

#[cfg(target_os="macos")]
os_mod!{macos}
// AppKit

#[cfg(target_os="ios")]
os_mod!{ios}
// UIKit

#[cfg(target_os="android")]
os_mod!{android}

#[cfg(target_os="emscripten")]
os_mod!{emscripten}

