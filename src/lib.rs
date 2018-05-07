//! DMC - Display and Multimedia Context
//! 
//! This is an attempt at an SDL2 rewrite in Rust. The end goal is to get
//! rid of the dependency on SDL2's shared library for Rust applications.

#![doc(html_root_url = "https://docs.rs/dmc/0.2.0")]
#![cfg_attr(nightly, feature(optin_builtin_traits))] // !Send, !Sync for Context
#![warn(missing_docs)]

#[allow(unused_imports)]
#[macro_use]
extern crate log;
extern crate vek;
extern crate uuid;

#[cfg(unix)]
#[macro_use]
extern crate nix;


/// Convenience shortcut for creating a `Context`.
pub fn init() -> error::Result<Context> {
    Context::new()
}

pub use vek::{
    Vec2, Extent2, Rect, Rgb, Rgba,
};

pub mod error;
pub use error::{ErrorKind, Error};
pub mod timeout;
pub use timeout::Timeout;
pub mod context;
pub use context::Context;
pub mod desktop;
pub use desktop::Desktop;
pub mod cursor;
pub use cursor::{Cursor, SystemCursor, RgbaCursorData, RgbaCursorAnimFrame};
pub mod window;
pub use window::{Window, WindowSettings, WindowTypeHint, NetWMWindowType};
pub mod hid;
pub mod event;
pub mod gl;
// pub mod battery;

mod version_cmp;

macro_rules! os_mod {
    ($os:ident) => {
        mod os {
            pub mod $os;
            pub use self::$os::{
                OsContext, OsWindow, OsWindowHandle, OsWindowFromHandleParams,
                OsCursor,
                OsGLPixelFormat, OsGLContext, OsGLProc,
                OsHidID,
                OsControllerState, OsControllerInfo,
                OsKeyboardState, OsKeycode, OsKeysym,
                OsMouseButtonsState,
                OsTabletPadButtonsState, OsTabletStylusButtonsState,
            };
        }

        // NOTE: This one is public on purpose!
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

