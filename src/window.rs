//! Window management.

use context::Context;
use vek::Extent2;
use os::OsWindow;
use gl::GLPixelFormat;
use error::Result;


impl Context {
    /// Attempts to create a new `Window` that satisfies given settings.
    pub fn create_window(&self, settings: &WindowSettings) -> Result<Window> {
        self.0.create_window(settings).map(Window)
    }
}



/// A wrapper around a platform-specific window.
#[derive(Debug)]
pub struct Window(pub(crate) OsWindow);

/// Full screen, or fixed-size.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum WindowMode {
    #[allow(missing_docs)]
    FixedSize(u32, u32),
    /// The window should take as much space on the desktop as possible.
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

impl<T: Into<Extent2<u32>>> From<T> for WindowMode {
    fn from(size: T) -> Self {
        let Extent2 { w, h } = size.into();
        WindowMode::FixedSize(w, h)
    }
}

