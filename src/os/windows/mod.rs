use std::os::raw::c_char;
use std::cmp::Ordering;
use std::time::Duration;
use std::ops::{Add, Sub, AddAssign, SubAssign};
use error::Result;
use desktop::Desktop;
use event::Event;
use timeout::Timeout;
use hint::Hint;
use window::{Window, WindowSettings, WindowHandle, WindowStyleHint, WindowTypeHint};
use {Vec2, Extent2, Rect, Rgba};


pub fn set_hint(hint: Hint) -> Result<()> {
    unimplemented!()
}

#[derive(Debug)]
pub struct OsContext;

impl OsContext {
    pub fn new() -> Result<Self> {
        unimplemented!()
    }
    pub fn untrap_mouse(&self) -> Result<()> {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct OsWindow;

impl OsContext {
    pub fn create_window(&self, settings: &WindowSettings) -> Result<OsWindow> {
        unimplemented!()
    }
    pub unsafe fn window_from_handle(&self, handle: OsWindowHandle, params: Option<&OsWindowFromHandleParams>) -> Result<OsWindow> {
        unimplemented!()
    }
}

impl OsContext {
    pub fn desktops(&self) -> Result<Vec<Desktop>> {
        unimplemented!()
    }
    pub fn current_desktop(&self) -> Result<usize> {
        unimplemented!()
    }
}


impl OsWindow {
    pub fn handle(&self) -> WindowHandle {
        unimplemented!()
    }
    pub fn set_title(&self, title: &str) -> Result<()> {
        unimplemented!()
    }
    pub fn title(&self) -> Result<String> {
        unimplemented!()
    }
    pub fn set_icon(&self, size: Extent2<u32>, data: &[Rgba<u8>]) -> Result<()> {
        unimplemented!()
    }
    pub fn icon(&self) -> Result<(Extent2<u32>, Vec<Rgba<u8>>)> {
        unimplemented!()
    }
    pub fn reset_icon(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn set_type_hint(&self, type_hint: &WindowTypeHint) -> Result<()> {
        unimplemented!()
    }
    pub fn set_style_hint(&self, style_hint: &WindowStyleHint) -> Result<()> {
        unimplemented!()
    }
    pub fn raise(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn set_min_size(&self, size: Extent2<u32>) -> Result<()> {
        unimplemented!()
    }
    pub fn set_max_size(&self, size: Extent2<u32>) -> Result<()> {
        unimplemented!()
    }
    pub fn set_resizable(&self, resizable: bool) -> Result<()> {
        unimplemented!()
    }
    pub fn is_resizable(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn set_movable(&self, movable: bool) -> Result<()> {
        unimplemented!()
    }
    pub fn is_movable(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn show(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn hide(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn toggle_visibility(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn is_visible(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn maximize(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn unmaximize(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn toggle_maximize(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn is_maximized(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn maximize_width(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn unmaximize_width(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn toggle_maximize_width(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn is_width_maximized(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn maximize_height(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn unmaximize_height(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn toggle_maximize_height(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn is_height_maximized(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn minimize(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn unminimize(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn toggle_minimize(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn is_minimized(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn enter_fullscreen(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn leave_fullscreen(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn toggle_fullscreen(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn is_fullscreen(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn demand_attention(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn demand_urgent_attention(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn position(&self) -> Result<Vec2<i32>> {
        unimplemented!()
    }
    pub fn set_position(&self, pos: Vec2<i32>) -> Result<()> {
        unimplemented!()
    }
    pub fn canvas_size(&self) -> Result<Extent2<u32>> {
        unimplemented!()
    }
    pub fn size(&self) -> Result<Extent2<u32>> {
        unimplemented!()
    }
    pub fn set_size(&self, size: Extent2<u32>) -> Result<()> {
        unimplemented!()
    }
    pub fn position_and_size(&self) -> Result<Rect<i32, u32>> {
        unimplemented!()
    }
    pub fn set_position_and_size(&self, r: Rect<i32, u32>) -> Result<()> {
        unimplemented!()
    }
    pub fn set_opacity(&self, alpha: f64) -> Result<()> {
        unimplemented!()
    }
    pub fn set_desktop(&self, i: usize) -> Result<()> {
        unimplemented!()
    }
    pub fn recenter_in_desktop(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn recenter_in_work_area(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn set_mouse_position(&self, pos: Vec2<i32>) -> Result<()> {
        unimplemented!()
    }
    pub fn mouse_position(&self) -> Result<Vec2<i32>> {
        unimplemented!()
    }
    pub fn trap_mouse(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn clear(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn clear_rect(&self, r: Rect<i32, u32>) -> Result<()> {
        unimplemented!()
    }
}

pub type OsWindowHandle = ();
pub type OsWindowFromHandleParams = ();

use cursor::{SystemCursor, RgbaCursorData, RgbaCursorAnimFrame};

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


pub type OsCursor = ();

#[derive(Debug)]
pub struct OsGLContext;
pub type OsGLPixelFormat = ();
pub type OsGLProc = ();

impl OsGLContext {
    pub unsafe fn get_proc_address(&self, name: *const c_char) -> Option<OsGLProc> {
        unimplemented!()
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct OsEventInstant;

impl PartialOrd for OsEventInstant {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unimplemented!()
    }
}

impl OsEventInstant {
    pub fn duration_since(&self, earlier: Self) -> Option<Duration> {
        assert!(self >= &earlier); // Normally already checked by EventInstant::duration_since
        unimplemented!()
    }
}
impl Add<Duration> for OsEventInstant {
    type Output = Self;
    fn add(self, rhs: Duration) -> Self {
        unimplemented!()
    }
}
impl Sub<Duration> for OsEventInstant {
    type Output = Self;
    fn sub(self, rhs: Duration) -> Self {
        unimplemented!()
    }
}
impl AddAssign<Duration> for OsEventInstant {
    fn add_assign(&mut self, rhs: Duration) {
        unimplemented!()
    }
}
impl SubAssign<Duration> for OsEventInstant {
    fn sub_assign(&mut self, rhs: Duration) {
        unimplemented!()
    }
}

impl OsContext {
    pub fn supports_raw_device_events(&self) -> Result<bool> {
        unimplemented!()
    }
    pub fn next_event(&self, timeout: Timeout) -> Option<Event> {
        unimplemented!()
    }
}



pub type OsDeviceID = ();
pub type OsAxisInfo = ();
pub type OsDeviceInfo = ();
pub type OsControllerState = ();
pub type OsControllerInfo = ();
pub type OsKeyboardState = ();
pub type OsKeycode = ();
pub type OsKeysym = ();
pub type OsMouseButtonsState = ();
pub type OsTabletInfo = ();
pub type OsTabletPadButtonsState = ();
pub type OsTabletStylusButtonsState = ();

pub mod device_consts {
    pub const MAX_THUMB_BUTTONS: Option<u32> = None;
    pub const MAX_TOP_BUTTONS: Option<u32> = None;
    pub const MAX_BASE_BUTTONS: Option<u32> = None;
    pub const MAX_NUM_BUTTONS: Option<u32> = None;
    pub const MAX_HAT_AXES: Option<u32> = None;
}