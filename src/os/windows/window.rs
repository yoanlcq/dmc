use error::Result;
use window::{Window, WindowSettings, WindowHandle, WindowStyleHint, WindowTypeHint};
use super::OsContext;
use {Vec2, Extent2, Rect, Rgba};

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
