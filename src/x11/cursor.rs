use std::os::raw::{c_uint, c_int};
use std::rc::Rc;
use std::mem;
use std::ptr;
use error::{Result, failed};
use cursor::{SystemCursor, RgbaCursorData, RgbaCursorAnimFrame};
use {Vec2, Extent2};
use super::context::{X11Context, X11SharedContext};
use super::window::X11SharedWindow;
use super::x11::xrender;
use super::x11::xlib as x;

#[derive(Debug)]
pub struct X11Cursor(pub Rc<X11SharedCursor>);

#[derive(Debug)]
pub struct X11SharedCursor {
    pub context: Rc<X11SharedContext>,
    pub x_cursor: x::Cursor,
    pub x_anim_cursors: Vec<xrender::XAnimCursor>,
}

impl Drop for X11SharedCursor {
    fn drop(&mut self) {
        let x_display = self.context.lock_x_display();
        unsafe {
            for x_anim_cursor in &self.x_anim_cursors {
                x::XFreeCursor(*x_display, x_anim_cursor.cursor);
            }
            x::XFreeCursor(*x_display, self.x_cursor);
        }
    }
}

impl X11SharedWindow {
    fn refresh_cursor_internal(&self) -> Result<()> {
        let x_display = self.context.lock_x_display();
        unsafe {
            if self.is_cursor_visible.get() {
                if let Some(ref user_cursor) = *self.user_cursor.borrow() {
                    x::XDefineCursor(*x_display, self.x_window, user_cursor.0.x_cursor);
                } else {
                    x::XUndefineCursor(*x_display, self.x_window);
                }
            } else {
                x::XDefineCursor(*x_display, self.x_window, self.context.invisible_x_cursor);
            }
        }
        self.context.x_flush();
        Ok(())
    }
    pub fn is_cursor_visible(&self) -> Result<bool> {
        Ok(self.is_cursor_visible.get())
    }
    pub fn toggle_cursor_visibility(&self) -> Result<()> {
        if self.is_cursor_visible.get() {
            self.hide_cursor()
        } else {
            self.show_cursor()
        }
    }
    pub fn hide_cursor(&self) -> Result<()> {
        self.is_cursor_visible.set(false);
        self.refresh_cursor_internal()
    }
    pub fn show_cursor(&self) -> Result<()> {
        self.is_cursor_visible.set(true);
        self.refresh_cursor_internal()
    }
    pub fn reset_cursor(&self) -> Result<()> {
        self.user_cursor.replace(None);
        self.refresh_cursor_internal()
    }
    pub fn set_cursor(&self, cursor: &X11Cursor) -> Result<()> {
        self.user_cursor.replace(Some(X11Cursor(Rc::clone(&cursor.0))));
        self.refresh_cursor_internal()
    }
    pub fn cursor(&self) -> Result<X11Cursor> {
        match *self.user_cursor.borrow() {
            Some(ref c) => Ok(X11Cursor(Rc::clone(&c.0))),
            None => {
                warn!("There is no reliable way to retrieve a windows's cursor on X11");
                let x_cursor = unsafe {
                    create_default_x_cursor(*self.context.lock_x_display())
                };
                Ok(X11Cursor(Rc::new(X11SharedCursor { context: Rc::clone(&self.context), x_cursor, x_anim_cursors: vec![] })))
            },
        }
    }
}

impl X11Context {
    pub fn create_system_cursor(&self, s: SystemCursor) -> Result<X11Cursor> {
        let x_cursor = unsafe {
            // Ignoring BadAlloc, BadValue
            x::XCreateFontCursor(*self.lock_x_display(), system_cursor_to_x_font_cursor_shape(s))
        };
        Ok(X11Cursor(Rc::new(X11SharedCursor { context: Rc::clone(&self.0), x_cursor, x_anim_cursors: vec![] })))
    }
    pub fn create_rgba_cursor(&self, frame: &RgbaCursorData) -> Result<X11Cursor> {
        let x_cursor = self.x_cursor_from_rgba(frame)?;
        Ok(X11Cursor(Rc::new(X11SharedCursor { context: Rc::clone(&self.0), x_cursor, x_anim_cursors: vec![] })))
    }

    pub fn create_animated_rgba_cursor(&self, frames: &[RgbaCursorAnimFrame]) -> Result<X11Cursor> {
        let mut errors = vec![];
        let mut x_anim_cursors: Vec<_> = frames.iter().map(|frame| {
            let duration_millis = {
                let secs = frame.duration.as_secs() as u64;
                let nano = frame.duration.subsec_nanos() as u64;
                secs*1000 + nano/1_000_000
            };
            let cursor = self.x_cursor_from_rgba(&frame.data)?;
            Ok(xrender::XAnimCursor { cursor, delay: duration_millis })
        }).inspect(|f: &Result<_>| if let Err(ref e) = *f {
            errors.push(Err(e.clone()));
        })
        .filter_map(Result::ok)
        .collect();

        let x_display = self.lock_x_display();

        if x_anim_cursors.len() < frames.len() {
            for x_anim_cursor in x_anim_cursors {
                unsafe {
                    x::XFreeCursor(*x_display, x_anim_cursor.cursor);
                }
            }
            return errors.swap_remove(0);
        }

        let x_cursor = unsafe {
            xrender::XRenderCreateAnimCursor(*x_display, x_anim_cursors.len() as _, x_anim_cursors.as_mut_ptr())
        };

        Ok(X11Cursor(Rc::new(X11SharedCursor { context: Rc::clone(&self.0), x_cursor, x_anim_cursors: vec![] })))
    }
}

impl X11SharedContext {
    pub fn best_cursor_size(&self, size_hint: Extent2<u32>) -> Result<Extent2<u32>> {
        let mut best = Extent2::<c_uint>::default();
        let drawable = self.x_default_root_window();
        let status = unsafe {
            // Ignoring BadDrawable
            x::XQueryBestCursor(
                *self.lock_x_display(), drawable,
                size_hint.w as _, size_hint.h as _,
                &mut best.w, &mut best.h
            )
        };
        if status != x::Success as _ {
            return failed(format!("XQueryBestCursor() returned {}", status));
        }
        Ok(best)
    }
    fn x_cursor_from_rgba(&self, frame: &RgbaCursorData) -> Result<x::Cursor> {
        // Return early if we don't have XRender, before allocating anything
        let xrender = self.xrender()?;

        let x_display = self.lock_x_display();
        let root = self.x_default_root_window();
        let visual = self.x_default_visual();
        let Extent2 { w, h } = frame.size;
        let Vec2 { x: hot_x, y: hot_y } = frame.hotspot;
        unsafe {
            let pix = x::XCreatePixmap(*x_display, root, w, h, 32);
            let pix_gc = x::XCreateGC(*x_display, pix, 0, ptr::null_mut());
            let pix_img = x::XCreateImage(
                *x_display, visual, 32, x::ZPixmap, 0,
                frame.rgba.as_ptr() as *const _ as *mut _,
                w, h, 32, 4*(w as c_int)
            );
            x::XPutImage(*x_display, pix, pix_gc, pix_img, 0, 0, 0, 0, w, h);
            let pic = xrender::XRenderCreatePicture(*x_display, pix, xrender.argb32_pict_format, 0, ptr::null_mut());
            let x_cursor = xrender::XRenderCreateCursor(*x_display, pic, hot_x as _, hot_y as _);
            xrender::XRenderFreePicture(*x_display, pic);
            x::XDestroyImage(pix_img);
            x::XFreeGC(*x_display, pix_gc);
            x::XFreePixmap(*x_display, pix);
            unimplemented!{"This code is probably wrong because data is RGBA but pict format is ARGB. Test me!!!"}
        }
    }
}

// FIXME: xlib_error::sync_catch() here
pub unsafe fn create_invisible_x_cursor(x_display: *mut x::Display) -> x::Cursor {
    let root = x::XDefaultRootWindow(x_display);
    let data = 0;
    let mut col: x::XColor = mem::zeroed();
    let pix = x::XCreateBitmapFromData(x_display, root, &data, 1, 1);
    let cur = x::XCreatePixmapCursor(x_display, pix, pix, &mut col, &mut col, 0, 0);
    x::XFreePixmap(x_display, pix);
    cur
}
pub unsafe fn create_default_x_cursor(x_display: *mut x::Display) -> x::Cursor {
    // Ignoring BadAlloc, BadValue
    x::XCreateFontCursor(x_display, XC_left_ptr)
}


pub fn system_cursor_to_x_font_cursor_shape(s: SystemCursor) -> c_uint {
    match s {
        SystemCursor::Arrow => XC_left_ptr,
        SystemCursor::Hand => XC_hand2,
        SystemCursor::Ibeam => XC_xterm,
        SystemCursor::Wait => XC_watch,
        SystemCursor::Crosshair => XC_tcross,
        SystemCursor::WaitArrow => XC_watch,
        SystemCursor::ResizeNWToSE => XC_fleur,
        SystemCursor::ResizeNEToSW => XC_fleur,
        SystemCursor::ResizeV => XC_sb_v_double_arrow,
        SystemCursor::ResizeH => XC_sb_h_double_arrow,
        SystemCursor::ResizeHV => XC_fleur,
        SystemCursor::Deny => XC_pirate,
        SystemCursor::Question => XC_question_arrow,
        SystemCursor::ReverseArrow => XC_right_ptr,
        SystemCursor::TopSide => XC_top_side,
        SystemCursor::BottomSide => XC_bottom_side,
        SystemCursor::LeftSide => XC_left_side,
        SystemCursor::RightSide => XC_right_side,
        SystemCursor::BottomLeftCorner => XC_bottom_left_corner,
        SystemCursor::BottomRightCorner => XC_bottom_right_corner,
        SystemCursor::TopLeftCorner => XC_top_left_corner,
        SystemCursor::TopRightCorner => XC_top_right_corner,
        SystemCursor::Pencil => XC_pencil,
        SystemCursor::Spraycan => XC_spraycan,
    }
}

macro_rules! xc_glyphs {
    ($($name:ident $val:tt)+) => {
        $(
            #[allow(non_upper_case_globals, dead_code)]
            pub const $name: c_uint = $val;
        )+
    };
}
xc_glyphs!{
    XC_num_glyphs 154
    XC_X_cursor 0
    XC_arrow 2
    XC_based_arrow_down 4
    XC_based_arrow_up 6
    XC_boat 8
    XC_bogosity 10
    XC_bottom_left_corner 12
    XC_bottom_right_corner 14
    XC_bottom_side 16
    XC_bottom_tee 18
    XC_box_spiral 20
    XC_center_ptr 22
    XC_circle 24
    XC_clock 26
    XC_coffee_mug 28
    XC_cross 30
    XC_cross_reverse 32
    XC_crosshair 34
    XC_diamond_cross 36
    XC_dot 38
    XC_dotbox 40
    XC_double_arrow 42
    XC_draft_large 44
    XC_draft_small 46
    XC_draped_box 48
    XC_exchange 50
    XC_fleur 52
    XC_gobbler 54
    XC_gumby 56
    XC_hand1 58
    XC_hand2 60
    XC_heart 62
    XC_icon 64
    XC_iron_cross 66
    XC_left_ptr 68
    XC_left_side 70
    XC_left_tee 72
    XC_leftbutton 74
    XC_ll_angle 76
    XC_lr_angle 78
    XC_man 80
    XC_middlebutton 82
    XC_mouse 84
    XC_pencil 86
    XC_pirate 88
    XC_plus 90
    XC_question_arrow 92
    XC_right_ptr 94
    XC_right_side 96
    XC_right_tee 98
    XC_rightbutton 100
    XC_rtl_logo 102
    XC_sailboat 104
    XC_sb_down_arrow 106
    XC_sb_h_double_arrow 108
    XC_sb_left_arrow 110
    XC_sb_right_arrow 112
    XC_sb_up_arrow 114
    XC_sb_v_double_arrow 116
    XC_shuttle 118
    XC_sizing 120
    XC_spider 122
    XC_spraycan 124
    XC_star 126
    XC_target 128
    XC_tcross 130
    XC_top_left_arrow 132
    XC_top_left_corner 134
    XC_top_right_corner 136
    XC_top_side 138
    XC_top_tee 140
    XC_trek 142
    XC_ul_angle 144
    XC_umbrella 146
    XC_ur_angle 148
    XC_watch 150
    XC_xterm 152
}

