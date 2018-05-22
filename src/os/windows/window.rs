use std::mem;
use std::ptr;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use error::Result;
use window::{Window, WindowSettings, WindowHandle, WindowStyleHint, WindowTypeHint};
use super::OsContext;
use super::winapi::{
    shared::{windef::*, minwindef::*,},
    um::{winuser::*, libloaderapi::*,},
};
use {Vec2, Extent2, Rect, Rgba};

#[derive(Debug)]
pub struct OsWindow;

extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unimplemented!()
}

fn winapi_errorcode_string(err: DWORD) -> String
{
    unimplemented!()
/*
    let messageBuffer: *mut u16 = ptr::null();
    let size = FormatMessageW(FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
                                 NULL, errorMessageID, MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT), (LPSTR)&messageBuffer, 0, NULL);

    std::string message(messageBuffer, size);

    //Free the buffer.
    LocalFree(messageBuffer);

    return message;
*/
}

impl OsContext {
    pub fn create_window(&self, settings: &WindowSettings) -> Result<OsWindow> {
        // TODO: CS_NOCLOSE
        // FIXME: CS_OWNDC only for OpenGL-enabled windows
        // FIXME: UnregisterClass, in case we're a DLL
        unsafe {
            let classname = {
                let mut classname: Vec<u16> = OsStr::new("Main DMC WNDCLASS").encode_wide().collect();
                classname.push(0);
                classname
            };
            assert!(classname.len() < 256);
            let hinstance = GetModuleHandleW(ptr::null());
            let wclass = WNDCLASSEXW {
                cbSize: mem::size_of::<WNDCLASSEXW>() as _,
                style: CS_DBLCLKS | CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: ptr::null_mut(),
                hIconSm: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: classname.as_ptr(),
            };
            let atom = RegisterClassExW(&wclass);
            assert_ne!(0, atom, "TODO GetLastError()");

            let Vec2 { x, y } = settings.position;
            let Extent2 { w, h } = settings.size;

            // Other nice style consts:
            // - WS_BORDER
            // - WS_CAPTION (title bar)
            // - WS_MAXIMIZEBOX (maximize button)
            // - WS_MINIMIZEBOX (minimize button)
            // - WS_SIZEBOX (same as WS_THICKFRAME) (sizing border)
            // - WS_SYSMENU (window menu on its title bar (WS_CAPTION must be specified))
            let ex_style = WS_EX_ACCEPTFILES | WS_EX_OVERLAPPEDWINDOW;
            let style = WS_OVERLAPPEDWINDOW;
            let hwnd = CreateWindowExW(
                ex_style,
                classname.as_ptr(),
                ptr::null(), // No title (yet)
                style,
                x, y, w as _, h as _,
                ptr::null_mut(), // No parent
                ptr::null_mut(), // No menu
                hinstance,
                ptr::null_mut(), // No custom data pointer
            );
            if hwnd.is_null() {
                // GetLastError(); // FIXME
            }
        }
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
