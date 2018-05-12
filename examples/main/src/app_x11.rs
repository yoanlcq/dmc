extern crate x11;
use dmc::Window;

use self::x11::xlib as x;
use std::ptr;

impl ::app::App {
    pub fn draw_image_window_x11(&self, window: &Window) {
        let x_display = self.context().unwrap().xlib_display();
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
