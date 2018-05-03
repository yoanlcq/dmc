//! System cursors and custom cursors.

use std::time::Duration;
use os::OsCursor;
use error::Result;
use context::Context;
use window::Window;
use super::{Vec2, Rgba, Extent2};

#[derive(Debug)]
pub struct Cursor(pub(crate) OsCursor);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum SystemCursor {
    Arrow,
    Hand,
    Ibeam,
    Wait,
    Crosshair,
    WaitArrow,
    ResizeNWToSE,
    ResizeNEToSW,
    ResizeV,
    ResizeH,
    ResizeHV,
    Deny,
    Question,
    ReverseArrow,
    TopSide,
    BottomSide,
    LeftSide,
    RightSide,
    BottomLeftCorner,
    BottomRightCorner,
    TopLeftCorner,
    TopRightCorner,
    Pencil,
    Spraycan,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct RgbaCursorAnimFrame {
    pub duration: Duration,
    pub data: RgbaCursorData,
}

/// FIXME: Use the `imgref` crate instead!
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct RgbaCursorData {
    pub hotspot: Vec2<u32>,
    pub size: Extent2<u32>,
    pub rgba: Vec<Rgba<u8>>,
}

impl Context {
    /// Creates a usable cursor from a well-known system cursor identifier.
    pub fn create_system_cursor(&self, s: SystemCursor) -> Result<Cursor> {
        self.0.create_system_cursor(s).map(Cursor)
    }
    /// Gets the best size for new cursors that is closest to `size_hint`.
    pub fn best_cursor_size(&self, size_hint: Extent2<u32>) -> Result<Extent2<u32>> {
        self.0.best_cursor_size(size_hint)
    }
    /// Creates a new alpha-blended cursor from RGBA data.
    pub fn create_rgba_cursor(&self, data: &RgbaCursorData) -> Result<Cursor> {
        self.0.create_rgba_cursor(data).map(Cursor)
    }
    /// Creates a new animated, alpha-blended cursor from RGBA frames.
    pub fn create_animated_rgba_cursor(&self, frames: &[RgbaCursorAnimFrame]) -> Result<Cursor> {
        self.0.create_animated_rgba_cursor(frames).map(Cursor)
    }
}

impl Window {
    /// Makes the cursor hidden as long as it stays within window.
    pub fn hide_cursor(&self) -> Result<()> {
        self.0.hide_cursor()
    }
    /// Undoes `hide_cursor()`.
    pub fn show_cursor(&self) -> Result<()> {
        self.0.show_cursor()
    }
    /// Is the cursor visible for this window?
    pub fn is_cursor_visible(&self) -> Result<bool> {
        self.0.is_cursor_visible()
    }
    /// Toggles cursor visibility for this window.
    pub fn toggle_cursor_visibility(&self) -> Result<()> {
        self.0.toggle_cursor_visibility()
    }
    /// Resets the cursor for this window to the platform-specific one.
    ///
    /// This doesn't have an effect on cursor visibility for this window.
    pub fn reset_cursor(&self) -> Result<()> {
        self.0.reset_cursor()
    }
    /// Sets the cursor for this window.
    pub fn set_cursor(&self, cursor: &Cursor) -> Result<()> {
        self.0.set_cursor(&cursor.0)
    }
    /// Gets the cursor defined for this window.
    ///
    /// X11-only: There is no way to retrieve a window's cursor, so
    /// if you haven't set a custom cursor for this window, a default
    /// cursor will be returned, but it might be wrong if a parent window's
    /// cursor is different.
    pub fn cursor(&self) -> Result<Cursor> {
        self.0.cursor().map(Cursor)
    }
}
