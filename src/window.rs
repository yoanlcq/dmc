//! Window management.

use context::Context;
use vek::{Vec2, Extent2, Rect, Rgba};
use os::{OsWindow, OsWindowHandle, OsWindowFromHandleParams};
use gl::GLPixelFormat;
use error::{self, Result};

impl Context {
    /// Attempts to create a new `Window` that satisfies given settings.
    pub fn create_window(&self, settings: &WindowSettings) -> Result<Window> {
        self.0.create_window(settings).map(Window)
    }
    /// Attempts to create a new `Window` from the given handle.
    ///
    /// This is unsafe because there's no guarantee that the handle is valid
    /// (i.e neither invalid nor outdated), and there's no guarantee that `params`
    /// is accurate.
    ///
    /// `params` should be `None` if `handle` refers to a window created via this crate.  
    /// Otherwise, it should be `Some` platform-specific information for configuring
    /// the foreign window.
    ///
    /// It is fine to drop the returned `Window` even if you previously constructed
    /// any number of others with the same handle; There is an internal reference
    /// count managed by the `Context`, and system resources are only freed when the
    /// last referring window is dropped.  
    /// This convenience is provided because manipulating windows via their handles
    /// is done quite often internally, and it's also handy if you don't feel like
    /// keeping track of a handle-to-window mapping yourself.  
    /// However you have to be careful if the underlying system window is "owned" 
    /// by someone else (including yourself or 3rd parties), in which case you should
    /// call `mem::forget()` on the last window in order to keep the system resources alive.
    pub unsafe fn window_from_handle(&self, handle: WindowHandle, params: Option<&OsWindowFromHandleParams>) -> Result<Window> {
        self.0.window_from_handle(handle.0, params).map(Window)
    }
}


/// Wrapper around a platform-specific window.
#[derive(Debug)]
pub struct Window(pub(crate) OsWindow);

/// Wrapper around a platform-specific window handle, which also acts as 
/// a lightweight ID.
///
/// This wrapper type only serves as a uniform facade and carries no implicit
/// meaning; it is only a promise that a handle is nothing more that
/// an integer or pointer, enforced by derived traits such as `Copy` and `Ord`.
///
/// In particular:
///
/// - The underlying handle may be invalid (e.g `INVALID_HANDLE` on Windows);
/// - The underlying handle may be outdated (refer to a window that was previously destroyed and
///   shouldn't be used anymore);
/// - There's no guarantee that the referred window was properly
///   configured for use by this crate's API.
///   This is especially important when you want to create a DMC `Window` using
///   your own platform-specific facilities.
// NOTE: Getters for this struct are in the respective platform-specific modules.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WindowHandle(pub(crate) OsWindowHandle);

impl From<OsWindowHandle> for WindowHandle {
    fn from(h: OsWindowHandle) -> Self {
        WindowHandle(h)
    }
}

/// A window type as defined by `_NET_WM_WINDOW_TYPE` in the [Extended Window Manager Hints (EWMH) specification](https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html).
///
/// The documentation for its variants is mostly copy-pasted from the linked specification.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum NetWMWindowType {
    /// A normal, top-level window.
    Normal,
    /// A desktop feature.  
    /// This can include a single window containing desktop icons with the same
    /// dimensions as the screen, allowing the desktop environment to have full control of the
    /// desktop, without the need for proxying root window clicks.
    Desktop,
    /// A dock or panel feature.  
    /// Typically a Window Manager would keep such windows on top of all
    /// other windows.
    Dock,
    /// A toolbar.
    Toolbar,
    /// A pinnable menu.
    Menu,
    /// A small persistent utility window, such as a palette or toolbox. It is distinct from type
    /// `Toolbar` because it does not correspond to a toolbar torn off from the main application.
    /// It's distinct from type `Dialog` because it isn't a transient dialog, the user will probably
    /// keep it open while they're working.
    Utility,
    /// A splash screen displayed as an application is starting up.
    Splash,
	/// A dialog window.
    Dialog,
    /// A dropdown menu, ie., the kind of menu that typically appears when the user clicks
    /// on a menubar, as opposed to a popup menu which typically appears when the user
    /// right-clicks on an object.
    DropdownMenu,
    /// A popup menu, ie., the kind of menu that typically appears when the user right clicks on an
    /// object, as opposed to a dropdown menu which typically appears when the user clicks on a
    /// menubar.
    PopupMenu,
    /// A tooltip, ie., a short piece of explanatory text that typically appears after the mouse
    /// cursor hovers over an object for a while.
    Tooltip,
    /// A notification.  
    /// An example of a notification would be a bubble appearing with informative text such as
    /// "Your laptop is running out of power", etc.
    Notification,
    /// A window that is popped up by combo boxes. An example is a window that appears below a text
    /// field with a list of suggested completions.
    Combo,
    /// The window is being dragged. Clients should set this hint when the window in question contains a representation of an object being dragged from one place to another. An example would be a window containing an icon that is being dragged from one file manager window to another. 
    DND,
}

/// Hint about the purpose or kind of a window.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct WindowTypeHint<'a> {
    /// A list of EWMH type hints, in order of preference (the first being the most preferrable).  
    /// It is probably wrong to leave it empty.
    pub net_wm: &'a [NetWMWindowType],
}

impl<'a> Default for WindowTypeHint<'a> {
    fn default() -> Self {
        Self {
            net_wm: &[NetWMWindowType::Normal],
        }
    }
}


/// Actually a simple thickness-color pair.
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Borders {
    /// Thickness, in pixels. If `None`, use the window manager's default.
    pub thickness: Option<u16>,
    /// If `None`, use the window manager's default.
    pub color: Option<Rgba<u8>>,
}

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct TitleBarFeatures {
    pub minimize: bool,
    pub maximize: bool,
    pub close: bool,
}

impl Default for TitleBarFeatures {
    fn default() -> Self {
        Self {
            minimize: true,
            maximize: true,
            close: true,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
/// Style hints for a window.
pub struct WindowStyleHint {
    /// If `None`, the window won't have a title bar.
    pub title_bar_features: Option<TitleBarFeatures>,
    /// If `None`, the window is borderless.
    pub borders: Option<Borders>,
}


/// The absolute minimum information a window needs at creation time.
#[derive(Debug)]
pub struct WindowSettings<'a> {
    /// The initial position (by top-left corner), in desktop space.
    pub position: Vec2<i32>,
    /// The initial size, in desktop space.
    pub size: Extent2<u32>,
    /// Support OpenGL ? (defaults to `None`).
    /// The settings need to be known beforehand so that the window
    /// can use the proper pixel format at the time of its creation.
    pub opengl: Option<&'a GLPixelFormat>,
    /// Some platforms (such as iOS and OS X) support high-dpi windows,
    /// which size in screen-coordinates then differ from their raster-
    /// coordinates size.
    /// 
    /// However this defaults to `false` because it might break some
    /// assumptions.
    pub high_dpi: bool,
}


impl Window {
    /// Gets this window's handle, which is an opaque wrapper around
    /// the lightweight platform-specific representation of a Window.
    pub fn handle(&self) -> WindowHandle {
        self.0.handle()
    }
    /// Sets the window's title to an UTF-8 string.
    pub fn set_title(&self, title: &str) -> Result<()> {
        self.0.set_title(title)
    }
    /// Gets the window's title as an UTF-8 string.
    pub fn title(&self) -> Result<String> {
        self.0.title()
    }
    /// FIXME: Use the `imgref` crate instead!
    /// Sets the window's icon via RGBA data.
    pub fn set_icon(&self, size: Extent2<u32>, data: &[Rgba<u8>]) -> Result<()> {
        self.0.set_icon(size, data)
    }
    /// FIXME: Use the `imgref` crate instead!
    /// Gets the window's icon as RGBA data.
    pub fn icon(&self) -> Result<(Extent2<u32>, Vec<Rgba<u8>>)> {
        self.0.icon()
    }
    /// Resets the window's icon to the system's default.
    pub fn reset_icon(&self) -> Result<()> {
        self.0.reset_icon()
    }
    /// Sets the window's type hint.
    ///
    /// **N.B: You normally set this property once, before
    /// showing the window for the first time.**  
    /// Doing otherwise might work, but it is not guaranteed because it is
    /// mostly dependant on the user's window manager.
    ///
    /// Also, there is no way to know whichs parts of the hint the
    /// window manager did take into account or apply.
    pub fn set_type_hint(&self, type_hint: &WindowTypeHint) -> Result<()> {
        self.0.set_type_hint(type_hint)
    }
    /// Sets the windows' style hint.
    pub fn set_style_hint(&self, style_hint: &WindowStyleHint) -> Result<()> {
        self.0.set_style_hint(style_hint)
    }
    /// Raises the window on top of the stack.  
    /// This should work even when the window is hidden.
    pub fn raise(&self) -> Result<()> {
        self.0.raise()
    }
    /// Constrains this window to have a size of at least the specified value (inclusive).
    pub fn set_min_size(&self, size: Extent2<u32>) -> Result<()> {
        self.0.set_min_size(size)
    }
    /// Constrains this window to have a size of at most the specified value (inclusive).
    pub fn set_max_size(&self, size: Extent2<u32>) -> Result<()> {
        self.0.set_max_size(size)
    }
    /// Allow or disallow resizing this window.
    ///
    /// The same effect can be achieved by calling both `set_min_size()` and `set_max_size()`
    /// with a same value, but "resizability" is yet a separate state.
    pub fn set_resizable(&self, resizable: bool) -> Result<()> {
        self.0.set_resizable(resizable)
    }
    /// Is this window resizable?
    pub fn is_resizable(&self) -> Result<bool> {
        self.0.is_resizable()
    }
    /// Allow or disallow moving this window.
    pub fn set_movable(&self, movable: bool) -> Result<()> {
        self.0.set_movable(movable)
    }
    /// Is this window movable?
    pub fn is_movable(&self) -> Result<bool> {
        self.0.is_movable()
    }
    /// Shows the window.
    ///
    /// When a window is first created, it is not shown yet.  
    /// This gives you the opportunity to configure it further
    /// (e.g setting its title) before showing it to the user.
    pub fn show(&self) -> Result<()> {
        self.0.show()
    }
    /// Devicees the window.
    ///
    /// This is not to be confused with _minimizing_ the window.  
    /// A hidden window isn't supposed to appear on the user's task bar.  
    /// In effect, this is somewhat similar to "closing" the window but keeping it
    /// in memory so that it can be shown ("reopened") again later.
    pub fn hide(&self) -> Result<()> {
        self.0.hide()
    }
    /// Toggles the window's visibility.
    pub fn toggle_visibility(&self) -> Result<()> {
        self.0.toggle_visibility()
    }
    /// Is the window visible?
    pub fn is_visible(&self) -> Result<bool> {
        self.0.is_visible()
    }
    /// Maximizes the window, so that it takes as much space on the desktop as possible.
    pub fn maximize(&self) -> Result<()> {
        self.0.maximize()
    }
    /// Undoes the last `maximize()`, reverting the window's position and size.
    pub fn unmaximize(&self) -> Result<()> {
        self.0.unmaximize()
    }
    /// Cycles between "maximized" and "unmaximized" states.
    pub fn toggle_maximize(&self) -> Result<()> {
        self.0.toggle_maximize()
    }
    /// Is the window maximized?
    pub fn is_maximized(&self) -> Result<bool> {
        self.0.is_maximized()
    }
    /// Maximizes the window's width, so that it takes as much space on the desktop as possible.
    pub fn maximize_width(&self) -> Result<()> {
        self.0.maximize_width()
    }
    /// Undoes the last `maximize_width()`, reverting the window's position and size.
    pub fn unmaximize_width(&self) -> Result<()> {
        self.0.unmaximize_width()
    }
    /// Cycles between "maximized width" and "unmaximized width" states.
    pub fn toggle_maximize_width(&self) -> Result<()> {
        self.0.toggle_maximize_width()
    }
    /// Is this window maximized horizontally?
    pub fn is_width_maximized(&self) -> Result<bool> {
        self.0.is_width_maximized()
    }
    /// Maximizes the window's height, so that it takes as much space on the desktop as possible.
    pub fn maximize_height(&self) -> Result<()> {
        self.0.maximize_height()
    }
    /// Undoes the last `maximize_height()`, reverting the window's position and size.
    pub fn unmaximize_height(&self) -> Result<()> {
        self.0.unmaximize_height()
    }
    /// Cycles between "maximized height" and "unmaximized height" states.
    pub fn toggle_maximize_height(&self) -> Result<()> {
        self.0.toggle_maximize_height()
    }
    /// Is this window maximized vertically?
    pub fn is_height_maximized(&self) -> Result<bool> {
        self.0.is_height_maximized()
    }
    /// Minimizes (iconifies) the window to task bar.
    pub fn minimize(&self) -> Result<()> {
        self.0.minimize()
    }
    /// Undoes the last `minimize()`, restoring the window.
    pub fn unminimize(&self) -> Result<()> {
        self.0.unminimize()
    }
    /// Toggles "minimized" state.
    pub fn toggle_minimize(&self) -> Result<()> {
        self.0.toggle_minimize()
    }
    /// Is the window minimized?
    pub fn is_minimized(&self) -> Result<bool> {
        self.0.is_minimized()
    }
    /// Makes the window full-screen.
    ///
    /// **N.B:** If the window was minimized, the behaviour is platform-dependant.  
    /// This method won't call `unminimize()` on your behalf.
    pub fn enter_fullscreen(&self) -> Result<()> {
        self.0.enter_fullscreen()
    }
    /// Exits full-screen mode, restoring the window's previous position and size.
    pub fn leave_fullscreen(&self) -> Result<()> {
        self.0.leave_fullscreen()
    }
    /// Toggles full-screen state.
    pub fn toggle_fullscreen(&self) -> Result<()> {
        self.0.toggle_fullscreen()
    }
    /// Is the window full-screen?
    pub fn is_fullscreen(&self) -> Result<bool> {
        self.0.is_fullscreen()
    }
    /// Softly demands the user's attention, in a platform-specific way.
    ///
    /// You should use this when you want your application to signal that,
    /// for instance, some task is complete, but not necessarily when it
    /// needs urgent attention.
    ///
    /// On X11, this is done by adding `_NET_WM_STATE_DEMANDS_ATTENTION`
    /// to the window's `_NET_WM_STATE`.  
    /// The effect is window-manager-dependant.
    pub fn demand_attention(&self) -> Result<()> {
        self.0.demand_attention()
    }
    /// Strongly demands the user's attention, in a platform-specific way.
    ///
    /// On X11, this is done by combining `demand_attention()` with 
    /// `XSetWMHints()` with the `XUrgencyHint` flag set.
    pub fn demand_urgent_attention(&self) -> Result<()> {
        self.0.demand_urgent_attention()
    }
    /// Retrieves the window's top-left corner position, in desktop pixel coordinates.
    pub fn position(&self) -> Result<Vec2<i32>> {
        self.0.position()
    }
    /// Moves the window in desktop space by its top-left corner.
    pub fn set_position(&self, pos: Vec2<i32>) -> Result<()> {
        self.0.set_position(pos)
    }
    /// Retrieves the size of the window's canvas, in raster-space pixel coordinates.
    /// 
    /// On High-DPI-enabled windows, it should be bigger
    /// than the size in desktop coordinates.  
    /// This is what you should use for pixel-perfect rendering.
    pub fn canvas_size(&self) -> Result<Extent2<u32>> {
        self.0.canvas_size()
    }
    /// Retrieves the window's size, in desktop pixel coordinates.
    /// 
    /// You should not rely on this being equal to its size
    /// in raster-space coordinates.  
    /// If you're interested in the "canvas"'s dimensions, 
    /// use the `canvas_size()` method instead.
    pub fn size(&self) -> Result<Extent2<u32>> {
        self.0.size()
    }
    /// Resizes the window in desktop space.
    pub fn set_size(&self, size: Extent2<u32>) -> Result<()> {
        self.0.set_size(size)
    }
    /// Retrieves the window's top-left position and its size, in desktop pixel coordinates.
    pub fn position_and_size(&self) -> Result<Rect<i32, u32>> {
        self.0.position_and_size()
    }
    /// Moves and resizes the window in desktop space.
    pub fn set_position_and_size(&self, r: Rect<i32, u32>) -> Result<()> {
        self.0.set_position_and_size(r)
    }
    /// Sets the window's overall opacity.
    ///
    /// The result is platform-specific and window-manager-specific.  
    /// Some window managers may simply ignore it, and some don't support it and
    /// won't report an error.  
    /// If it doesn't appear to work, go to your window manager's settings and make
    /// sure compositing is enabled. Because of the wide variety of window managers
    /// in the wild, it is impossible to programmatically enable compositing in a portable
    /// and future-proof way, so this method won't attempt to do it on your behalf
    /// (besides, this is a decision that must be left to the user).
    ///
    /// The `alpha` value is clamped to be between 0 and 1, if necessary.  
    /// This method fails if `alpha` is `NaN`.
    ///
    /// On X11, this property is set via `_NET_WM_WINDOW_OPACITY` and the computed value
    /// is `alpha * u32::MAX`, which is also why `alpha` is an `f64`
    /// because an `f32`'s mantissa isn't wide enough to yield an accurate result in this
    /// case.
    pub fn set_opacity(&self, alpha: f64) -> Result<()> {
        if alpha.is_nan() {
            return error::invalid_arg("alpha can't be NaN!");
        }
        self.0.set_opacity(alpha)
    }
    /// Attempts to move the window to the desktop specified by index `i`.
    ///
    /// Some platforms might not support this.
    pub fn set_desktop(&self, i: usize) -> Result<()> {
        self.0.set_desktop(i)
    }
    /// Attempts to recenter the window in desktop space.
    pub fn recenter_in_desktop(&self) -> Result<()> {
        self.0.recenter_in_desktop()
    }
    /// Attempts to recenter the window in the desktop's work area, that is,
    /// the advised zone that excludes panels, task bars, etc.
    pub fn recenter_in_work_area(&self) -> Result<()> {
        self.0.recenter_in_work_area()
    }
    /// Warps the main cursor's position to the given window-relative position.
    pub fn set_mouse_position(&self, pos: Vec2<i32>) -> Result<()> {
        self.0.set_mouse_position(pos)
    }
    /// Queries the main cursor's position in window coordinates.
    pub fn mouse_position(&self) -> Result<Vec2<i32>> {
        self.0.mouse_position()
    }
    /// Attemps to trap the mouse inside this window.
    ///
    /// If it succeeds, the window first receives mouse focus, then the main cursor
    /// is constrained to always stay within the window as long as the trap is active.
    ///
    /// This may fail if the window is hidden.
    ///
    /// To undo a mouse trap, use `Context::untrap_mouse()`.
    ///
    /// **Tip**: Here is how you would trap the mouse for, say, a FPS game.
    /// - `window.trap_mouse()`, so that mouse is confined to the window;
    /// - `window.hide_cursor()`;
    /// - Only interpret relative mouse motion events. _(implementation note: XI2 RawMotion
    /// events)_
    /// - Optionally, `window.set_mouse_position(center)` every frame, but it's not much use.
    pub fn trap_mouse(&self) -> Result<()> {
        self.0.trap_mouse()
    }
}
