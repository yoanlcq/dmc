use dmc::Context;

pub fn early() {
    setup_env();
    setup_log();

    #[cfg(any(target_os="linux", target_os="freebsd", target_os="dragonfly", target_os="openbsd", target_os="netbsd"))]
    x11_specific::early_x11();
}

fn setup_env() {
    //env::set_var("RUST_LOG", "info");
    ::std::env::set_var("RUST_BACKTRACE", "full");
}

fn setup_log() {
    use ::std::io::Write;

    let mut builder = ::env_logger::Builder::new();
    builder.format(|buf, record| {
        let s = format!("{}", record.level());
        let s = s.chars().next().unwrap();
        writeln!(buf, "[{}] {}", s, record.args())
    }).filter(None, ::log::LevelFilter::Debug);

    if let Ok(rust_log) = ::std::env::var("RUST_LOG") {
        builder.parse(&rust_log);
    }
    builder.init();
}

#[cfg(any(target_os="linux", target_os="freebsd", target_os="dragonfly", target_os="openbsd", target_os="netbsd"))]
mod x11_specific {
    extern crate x11;

    use super::*;
    use std::ptr;

    pub fn early_x11() {
        drop(Context::with_x11_display_name(None).unwrap());
        let name = ::std::ffi::CStr::from_bytes_with_nul(b":0.0\0").unwrap();
        drop(Context::with_x11_display_name(Some(name)).unwrap());
        let dpy = unsafe {
            x11::xlib::XOpenDisplay(ptr::null())
        };
        drop(Context::with_xlib_display(dpy).unwrap());
        // NOTE: Don't do XCloseDisplay(dpy), the context takes ownership of it!
    }
}
