use cursor::{SystemCursor, RgbaCursorData, RgbaCursorAnimFrame};
use error::Result;
use super::{OsContext, OsWindow};
use Extent2;

pub type OsCursor = ();

impl OsContext {
    pub fn create_system_cursor(&self, s: SystemCursor) -> Result<OsCursor> {
        unimplemented!()
    }
    pub fn best_cursor_size(&self, size_hint: Extent2<u32>) -> Result<Extent2<u32>> {
        unimplemented!()
    }
    pub fn create_rgba_cursor(&self, data: &RgbaCursorData) -> Result<OsCursor> {
        unimplemented!()
    }
    pub fn create_animated_rgba_cursor(&self, frames: &[RgbaCursorAnimFrame]) -> Result<OsCursor> {
        unimplemented!()
    }
}

impl OsWindow {
    pub fn hide_cursor(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn show_cursor(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn is_cursor_visible(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn toggle_cursor_visibility(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn reset_cursor(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn set_cursor(&self, cursor: &OsCursor) -> Result<()> {
        unimplemented!()
    }
    pub fn cursor(&self) -> Result<OsCursor> {
        unimplemented!()
    }
}
