use dmc::{Context, Window};

pub fn early() {
    setup_env();
    setup_log();

    #[cfg(any(target_os="linux", target_os="freebsd", target_os="dragonfly", target_os="openbsd", target_os="netbsd"))]
    x11_specific::early_x11();
}

fn setup_env() {
    ::std::env::set_var("RUST_LOG", "trace");
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
pub mod x11_specific {
    extern crate x11;

    use self::x11::xlib as x;

    use super::*;
    use std::ptr;

    pub fn early_x11() {
        drop(Context::with_x11_display_name(None).unwrap());
        let name = ::std::ffi::CStr::from_bytes_with_nul(b":0.0\0").unwrap();
        drop(Context::with_x11_display_name(Some(name)).unwrap());
        let dpy = unsafe {
            x::XOpenDisplay(ptr::null())
        };
        drop(unsafe {Context::from_xlib_display(dpy)}.unwrap());
        // NOTE: Don't do XCloseDisplay(dpy), the context takes ownership of it!
    }

    pub fn clear_window_x11(window: &Window) {
        let x_display = window.xlib_display();
        let x_window = window.handle().x_window();
        unsafe {
            x::XClearWindow(x_display, x_window);
            x::XSync(x_display, x::False);
        }
    }

    pub fn draw_image_window_x11(window: &Window) {
        let x_display = window.xlib_display();
        let x_window = window.handle().x_window();
        unsafe {
            let visual = x::XDefaultVisual(x_display, x::XDefaultScreen(x_display));
            let depth = 32;
            let format = x::ZPixmap;
            let offset = 0;
            let data = ptr::null_mut();
            let bitmap_pad = 32;
            let bytes_per_line = 0; // OK if pixels are contiguous in memory
            let (w, h) = (0, 0);
            let (sx, sy, dx, dy) = (0, 0, 0, 0);
            let img = x::XCreateImage(
                x_display, visual, depth, format,
                offset, data, w, h, bitmap_pad, bytes_per_line
            );
            let valuemask = 0;
            let values = ptr::null_mut();
            let gc = x::XCreateGC(x_display, x_window, valuemask, values);
            x::XPutImage(x_display, x_window, gc, img, sx, sy, dx, dy, w, h);
            x::XFreeGC(x_display, gc);
            x::XDestroyImage(img);
        }
    }
}
