use std::rc::Rc;
use gl::*;
use os::*;
use context::Error;
use super::{Extent2, Vec2, Rgba};
use decision::Decision;
use cursor::Cursor;
use image::Image;

pub type Icon = Image<Rgba<u8>>;

#[derive(Debug)]
pub struct Window {
    pub(crate) os_window: OsWindow,
    pub(crate) fps_limit: Option<f32>,
}

/// Full screen, or fixed-size.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum WindowMode {
    #[allow(missing_docs)]
    FixedSize(Extent2<u32>),
    #[allow(missing_docs)]
    Maximized,
    #[allow(missing_docs)]
    FullScreen,
    // This one is removed because there's no use case for it and can't
    // easily be implemented.
    // /// This is _possible_ but I wonder who would use this.
    // FixedSizeFullScreen(Extent2<u32>),
}

/// The absolute minimum information a window needs at creation time.
/// 
/// The `Default` implementation picks the most permissive values, except
/// for `fully_opaque` which is set to `true`, because people seldom
/// need semi-transparent windows.
#[derive(Debug)]
pub struct WindowSettings<'a> {
    /// Specifies whether you want a full-screen or fixed-size window.
    /// The default value is a `FixedSize` obtained by a heuristic
    /// based on the desktop's available size, which picks a size
    /// that leaves reasonable space around the window.
    pub mode: WindowMode,
    /// Support OpenGL ? (defaults to `None`).
    /// The settings need to be known beforehand so that the window
    /// can use the proper pixel format at the time of its creation.
    pub opengl: Option<&'a GLPixelFormat>,
    /// `true` by default -
    /// If `false`, the window won't be resizable, not even manually by
    /// the user. Also keep in mind that some targets (other than 
    /// desktop) don't support resizing at all, in which case this flag
    /// is silently ignored.
    pub resizable: bool,
    /// Some platforms (such as iOS and OS X) support high-dpi windows,
    /// which size in screen-coordinates then differ from their raster-
    /// coordinates size.
    /// 
    /// However this defaults to `false` because it might break some
    /// assumptions.
    pub allow_high_dpi: bool,
    /// Some windowing systems support semi-transparent windows, which
    /// is useful for making desktop companions, however it's better to
    /// let them know beforehand that you need such a feature.  
    /// This defaults to `true` because this is commonly expected.
    pub fully_opaque: bool,
}

impl Default for WindowMode {
    fn default() -> Self {
        WindowMode::Maximized
    }
}

impl<'a> Default for WindowSettings<'a> {
    fn default() -> Self {
        Self {
            opengl: None,
            resizable: true,
            allow_high_dpi: true,
            fully_opaque: true,
            mode: Default::default(),
        }
    }
}

impl<T: Into<Extent2<u32>>> From<T> for WindowMode {
    fn from(size: T) -> Self {
        WindowMode::FixedSize(size.into())
    }
}

impl<'a, T: Into<Extent2<u32>>> From<T> for WindowSettings<'a> {
    fn from(size: T) -> Self {
        Self {
            mode: WindowMode::from(size),
            .. Default::default()
        }
    }
}

/// Actually a simple thickness-color pair.
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Borders {
    /// Thickness, in pixels. If `Auto`, use the window manager's default.
    pub thickness: Decision<u16>,
    /// If `Auto`, use the window manager's default.
    pub color: Decision<Rgba<u8>>,
}

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct TitleBarStyle {
    pub minimize_button: bool,
    pub maximize_button: bool,
    pub close_button: bool,
}

impl Default for TitleBarStyle {
    fn default() -> Self {
        Self {
            minimize_button: true,
            maximize_button: true,
            close_button: true,
        }
    }
}

/// Style hints for a window.
pub struct WindowStyle {
    /// If `None`, the window won't have a title bar.
    pub title_bar: Option<TitleBarStyle>,
    /// If `None`, the window is borderless.
    pub borders: Option<Borders>,
}


impl Window {
    /// A window won't appear until this method is called.
    ///
    /// Likewise, swapping OpenGL buffers (i.e "presenting") has
    /// no effect on a window that is not visible. 
    /// At least on X11, if you try to present on a window and then show
    /// it, you'll just get garbage instead.
    ///
    /// On X11, this function waits for the server to process the request.
    /// If it didn't, there would be a small chance that swapping buffers
    /// happen before showing the window, even if you did the operations
    /// in the correct order.
    pub fn show(&self) -> Result<(), Error> { self.os_window.show() }
    /// The obvious reciprocal of `show()`.
    pub fn hide(&self) -> Result<(), Error> { self.os_window.hide() }

    /// Sets the window's title.
    pub fn set_title(&self, title: &str) -> Result<(), Error> {
        self.os_window.set_title(title)
    }
    #[allow(missing_docs)]
    pub fn set_icon(&self, icon: Icon) -> Result<(), Error> {
        self.os_window.set_icon(icon)
    }
    #[allow(missing_docs)]
    pub fn clear_icon(&self) -> Result<(), Error> {
        self.os_window.clear_icon()
    }
    /// Attempts to set the window's borders.
    pub fn set_style(&self, style: &WindowStyle) -> Result<(), Error> {
        self.os_window.set_style(style)
    }
    /// Centers a window relatively to the space it is in, with regards to
    /// its size.
    pub fn recenter(&self) -> Result<(), Error> {
        self.os_window.recenter()
    }

    /// Sets the window's opacity, provided the window was created with
    /// the `fully_opaque` flag set to `false`.
    /// 
    /// Valid values for `opacity` range from 0 to 1 (both inclusive).  
    /// You're expected to clamp the value yourself if needed.
    pub fn set_opacity(&self, opacity: f32) -> Result<(), Error> {
        self.os_window.set_opacity(opacity)
    }


    /*
    /// Retrieves the window's internal implementation details, if you
    /// need to work around missing features.  
    /// 
    /// If that happens, you are welcome to report an issue!
    pub unsafe fn get_internal(&self) -> Rc<OsWindow> {
        &self.os_window
    }
    */

    /// The window's size, in screen coordinates.
    /// 
    /// You should not rely on this being equal to its size
    /// in raster-space coordinates.  
    /// If you're interested in the "canvas"'s dimensions, 
    /// use the `query_canvas_size()` method instead.
    /// 
    /// The `query()` part means that the operation is possibly heavy and
    /// the result is not implicitly cached:  
    /// it's your responsibility to do so if this is what you want.
    pub fn query_screenspace_size(&self) -> Extent2<u32> {
        self.os_window.query_screenspace_size()
    }
    /// The window's size, in raster-space coordinates.  
    /// 
    /// On High-DPI-enabled windows, it should be bigger
    /// than the size in screen-coordinates.  
    /// This is what you should use for pixel-perfect rendering.
    /// 
    /// The `query()` part means that the operation is possibly heavy and
    /// the result is not implicitly cached:  
    /// it's your responsibility to do so if this is what you want.
    pub fn query_canvas_size(&self) -> Extent2<u32> {
        self.os_window.query_canvas_size()
    }


    /// Attempts to maximize the window (as in, take as much space as
    /// possible).
    pub fn maximize(&self) -> Result<(), Error> { self.os_window.maximize() }
    pub fn unmaximize(&self) -> Result<(), Error> { self.os_window.unmaximize() }
    pub fn toggle_maximize(&self) -> Result<(), Error> { self.os_window.toggle_maximize() }
    /// Attempts to minimize the window (as in, minimize to task bar).
    pub fn minimize(&self) -> Result<(), Error> { self.os_window.minimize() }
    /// The reciprocal of `minimize()`.
    pub fn restore(&self) -> Result<(), Error> { self.os_window.restore() }
    /// Attempts to set the window on top of the stack and request focus.
    pub fn raise(&self) -> Result<(), Error> { self.os_window.raise() }
    /// Attempts to go full-screen.
    /// 
    /// The `Window` struct doesn't keep track of an `is_fullscreen`
    /// boolean: it is yours to manage if you need one. This method
    /// won't perform the checks for you.
    /// However, for convenience, it saves the window's current size
    /// to automatically restore it whenever leaving full-screen mode.
    pub fn enter_fullscreen(&self) -> Result<(), Error> { self.os_window.enter_fullscreen() }
    /// Attempts to leave full-screen mode.
    /// 
    /// See `enter_fullscreen()`.
    pub fn leave_fullscreen(&self) -> Result<(), Error> { self.os_window.leave_fullscreen() }
    pub fn toggle_fullscreen(&self) -> Result<(), Error> { self.os_window.toggle_fullscreen() }

    /// Unconditionnally prevents the window's size from going below the
    /// given threshold.
    pub fn set_minimum_size(&self, size: Extent2<u32>) -> Result<(), Error> {
        self.os_window.set_minimum_size(size)
    }
    /// Unconditionnally prevents the window's size from going above the
    /// given threshold.
    pub fn set_maximum_size(&self, size: Extent2<u32>) -> Result<(), Error> {
        self.os_window.set_maximum_size(size)
    }
    /// Moves the window to the given absolute position in 
    /// desktop-space.  
    /// 
    /// The anchor is the window's top-left corner.
    pub fn position(&self) -> Result<Vec2<i32>, Error> {
        self.os_window.position()
    }
    /// Moves the window to the given absolute position in 
    /// desktop-space.  
    /// 
    /// The anchor is the window's top-left corner.
    pub fn set_position(&self, pos: Vec2<i32>) -> Result<(), Error> {
        self.os_window.set_position(pos)
    }
    /// Attempts to set the window's screen-space size.
    pub fn resize(&self, size: Extent2<u32>) -> Result<(), Error> {
        self.os_window.resize(size)
    }
    pub fn show_cursor(&self) -> Result<(), Error> {
        self.os_window.show_cursor()
    }
    pub fn hide_cursor(&self) -> Result<(), Error> {
        self.os_window.hide_cursor()
    }
    pub fn set_cursor(&self, cursor: Rc<Cursor>) -> Result<(), Error> {
        self.os_window.set_cursor(cursor)
    }
    pub fn set_cursor_position(&self, pos: Vec2<u32>) -> Result<(), Error> {
        self.os_window.set_cursor_position(pos)
    }
    pub fn query_cursor_position(&self) -> Result<Vec2<u32>, Error> {
        self.os_window.query_cursor_position()
    }


    pub fn demand_attention(&self) -> Result<(), Error> {
        self.os_window.demand_attention()
    }

    /// Lowers to the plaftorm-specific "<xxglxx>ContextMakeCurrent()".
    /// Please note that making a context current is a thread-wide operation.
    ///
    /// FIXME: There's no way to prevent SwapChains from co-existing.
    /// There's no mechanism to prevent users from using a SwapChain that was
    /// created before another "make_current".
    pub fn make_gl_context_current(&mut self, gl_context: Option<&GLContext>) {
        self.os_window.make_gl_context_current(gl_context.map(|x| &x.0));
        // TODO: store in window that is it OK to swap buffers.
        if self.set_gl_swap_interval(Default::default()).is_err() {
            self.set_gl_swap_interval(GLSwapInterval::LimitFps(60_f32)).unwrap();
        }
    }

    /// Lowers to the plaftorm-specific `XXglXXSwapBuffers()`.
    /// Use this when you're done rendering the current frame.
    /// 
    /// Quoting SDL2's docs:  
    /// On Mac OS X make sure you bind 0 to the draw framebuffer before 
    /// swapping the window,
    /// otherwise nothing will happen. See [this blog
    /// post](http://renderingpipeline.com/2012/05/nsopenglcontext-flushbuffer-might-not-do-what-you-think/) for more info.
    pub fn present_gl(&self) {
        match self.fps_limit {
            None => self.os_window.gl_swap_buffers(),
            Some(_fps_limit) => {
                // TODO: Implement fixed time-step
                unimplemented!{"Limiting FPS isn't supported yet."}
            },
        }
    }

    /// Attempts to set the chain's swap interval. 
    pub fn set_gl_swap_interval(&mut self, interval: GLSwapInterval) -> Result<(),Error> {
        self.fps_limit = None;
        if let GLSwapInterval::LimitFps(fps_limit) = interval {
            self.fps_limit = Some(fps_limit);
            return Ok(());
        }
        self.os_window.set_gl_swap_interval(interval)
    }
}
