use std::rc::Rc;
use std::ptr;
use cursor::{SystemCursor, RgbaCursorData, RgbaCursorAnimFrame};
use error::{Result, failed};
use super::{OsSharedContext, OsSharedWindow, winapi_utils::{self as w32, *}};
use Extent2;

#[derive(Debug, Hash)]
pub struct HCursor(pub HCURSOR);

impl Drop for HCursor {
    fn drop(&mut self) {
        // XXX: Do we need to drop a HCURSOR??
    }
}

#[derive(Debug, Hash)]
pub struct OsCursor(pub Rc<HCursor>);


fn system_cursor_resid(s: SystemCursor) -> Option<LPCWSTR> {
    Some(match s {
        SystemCursor::Arrow => w32::IDC_ARROW,
        SystemCursor::UpArrow => w32::IDC_UPARROW,
        SystemCursor::Hand => w32::IDC_HAND,
        SystemCursor::Ibeam => w32::IDC_IBEAM,
        SystemCursor::Wait => w32::IDC_WAIT,
        SystemCursor::Crosshair => w32::IDC_CROSS,
        SystemCursor::WaitArrow => w32::IDC_APPSTARTING,
        SystemCursor::ResizeNWToSE => w32::IDC_SIZENWSE,
        SystemCursor::ResizeNEToSW => w32::IDC_SIZENESW,
        SystemCursor::ResizeWE => w32::IDC_SIZEWE,
        SystemCursor::ResizeNS => w32::IDC_SIZENS,
        SystemCursor::ResizeAll => w32::IDC_SIZEALL,
        SystemCursor::Deny => w32::IDC_NO,
        SystemCursor::Question => w32::IDC_HELP,
        _ => return None,
    })
}

impl OsSharedContext {
    pub fn create_default_system_cursor(&self) -> Result<OsCursor> {
        self.create_system_cursor(SystemCursor::Arrow)
    }
    pub fn create_system_cursor(&self, s: SystemCursor) -> Result<OsCursor> {
        match system_cursor_resid(s) {
            Some(resid) => unsafe {
                let hcursor = w32::LoadCursorW(ptr::null_mut(), resid);
                Ok(OsCursor(Rc::new(HCursor(hcursor))))
            },
            None => failed(format!("Unsupported system cursor: {:?}", s)),
        }
    }
    pub fn best_cursor_size(&self, _size_hint: Extent2<u32>) -> Result<Extent2<u32>> {
        Ok(Extent2::new(32, 32))
    }
    pub fn create_rgba_cursor(&self, data: &RgbaCursorData) -> Result<OsCursor> {
        unimplemented!()
    }
    pub fn create_animated_rgba_cursor(&self, frames: &[RgbaCursorAnimFrame]) -> Result<OsCursor> {
        unimplemented!()
    }
}

impl OsSharedWindow {
    pub fn hide_cursor(&self) -> Result<()> {
        self.is_cursor_visible.set(false);
        Ok(())
    }
    pub fn show_cursor(&self) -> Result<()> {
        self.is_cursor_visible.set(true);
        Ok(())
    }
    pub fn is_cursor_visible(&self) -> Result<bool> {
        Ok(self.is_cursor_visible.get())
    }
    pub fn toggle_cursor_visibility(&self) -> Result<()> {
        self.is_cursor_visible.set(!self.is_cursor_visible.get());
        Ok(())
    }
    pub fn reset_cursor(&self) -> Result<()> {
        self.set_cursor(&self.context.create_default_system_cursor()?)
    }
    pub fn set_cursor(&self, cursor: &OsCursor) -> Result<()> {
        self.cursor.replace(Rc::clone(&cursor.0));
        Ok(())
    }
    pub fn cursor(&self) -> Result<OsCursor> {
        Ok(OsCursor(Rc::clone(&self.cursor.borrow())))
    }
}
