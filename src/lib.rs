//! DMC - Display and Multimedia Context
//! 
//! This is an attempt at an SDL2 rewrite in Rust. The end goal is to get
//! rid of the dependency on SDL2's DLL for Rust apps.

// New design attempt:
//
// - C'est le contexte qui owne tout : les fenêtres, les devices, etc.
//   Raison: Il a besoin d'établir le mapping.
// - Les borrows et les lifetimes sont chiants.
//   Utiliser des Rc, parce qu'on peut.
//   En plus les lifetimes des objets dépendent de la plateforme.
//   Un Event devrait être self-contained.
// - Il n'y a pas de solution parfaite
//   Les solutions existantes sont déjà des hacks. Pas thread-safe,
//   qui stockent des infos redondantes, etc.

#![doc(html_root_url = "https://docs.rs/dmc/0.2.0")]
#![feature(optin_builtin_traits)]

#[macro_use]
extern crate log;
extern crate libc;
extern crate vek;

pub use vek::{
    Vec2, Vec3, Vec4, Extent2, Extent3, Rgba, Rgb,
    Rect,
};

mod version_cmp;

pub mod error;
pub use error::{ErrorKind, Error};
pub mod context;
pub use context::Context;
pub mod window;
pub use window::{Window, WindowSettings, WindowMode};
pub mod gl;
pub mod cursor;
pub mod hid;
pub use hid::*;
pub mod event;
pub use event::{Event, Click};
pub mod battery;
pub use battery::{BatteryState, BatteryStatus};

#[cfg(target_os="linux")]
#[path="os/linux.rs"]
pub mod os;
#[cfg(any(target_os="freebsd", target_os="dragonfly", target_os="openbsd", target_os="netbsd"))]
#[path="os/bsd.rs"]
pub mod os;
#[cfg(target_os="windows")]
#[path="os/windows.rs"]
pub mod os;
#[cfg(target_os="macos")] /* AppKit */ 
#[path="os/macos.rs"]
pub mod os;
#[cfg(target_os="android")]
#[path="os/android.rs"]
pub mod os;
#[cfg(target_os="ios")] /* UIKit */ 
#[path="os/ios.rs"]
pub mod os;
#[cfg(target_os="winrt")]
#[path="os/winrt.rs"]
pub mod os;
#[cfg(target_os="emscripten")]
#[path="os/emscripten.rs"]
pub mod os;

pub use os::OsGLProc;
