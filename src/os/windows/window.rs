use std::ptr;
use std::rc::Rc;
use std::ops::Deref;
use error::{Result, failed};
use window::{Window, WindowSettings, WindowHandle, WindowStyleHint, WindowTypeHint, TitleBarFeatures, Borders};
use super::{OsContext, OsSharedContext, winapi_utils::*};
use {Vec2, Extent2, Rect, Rgba};


pub type OsWindowHandle = HWND;

#[derive(Debug)]
pub struct OsWindowFromHandleParams {
    pub class_atom: ATOM,
}

#[derive(Debug)]
pub struct OsSharedWindow {
    pub context: Rc<OsSharedContext>,
    pub class_atom: ATOM,
    pub hwnd: HWND,
}

#[derive(Debug)]
pub struct OsWindow(pub(crate) Rc<OsSharedWindow>);

impl Deref for OsWindow {
    type Target = OsSharedWindow;
    fn deref(&self) -> &OsSharedWindow {
        &self.0
    }
}

impl Drop for OsSharedWindow {
    fn drop(&mut self) {
        let &mut Self {
            ref context, class_atom, hwnd,
        } = self;
        unsafe {
            let is_ok = DestroyWindow(hwnd);
            let is_ok = UnregisterClassW(class_atom as _, context.hinstance());
        }
    }
}

impl OsContext {
    pub fn create_window(&self, settings: &WindowSettings) -> Result<OsWindow> {
        let &WindowSettings {
            position: Vec2 { x, y }, size: Extent2 { w, h }, ref opengl, high_dpi,
        } = settings;
        unsafe {
            let ex_style = WS_EX_ACCEPTFILES | WS_EX_OVERLAPPEDWINDOW;
            let style = WS_OVERLAPPEDWINDOW;
            let class_settings = super::context::ClassSettings {
                owndc: true, noclose: false,
            };
            let class_atom = self.get_or_register_class(&class_settings)?;
            let hwnd = CreateWindowExW(
                ex_style,
                MAKEINTATOM(class_atom),
                ptr::null(), // No title (yet)
                style,
                x, y, w as _, h as _,
                ptr::null_mut(), // No parent
                ptr::null_mut(), // No menu
                self.hinstance(),
                ptr::null_mut(), // No custom data pointer
            );
            if hwnd.is_null() {
                return winapi_fail("CreateWindowExW");
            }
            let os_window = OsSharedWindow {
                context: Rc::clone(&self.0),
                class_atom, hwnd
            };
            let os_window = Rc::new(os_window);
            self.weak_windows.borrow_mut().insert(hwnd, Rc::downgrade(&os_window));
            Ok(OsWindow(os_window))
        }
    }
    pub unsafe fn window_from_handle(&self, hwnd: OsWindowHandle, params: Option<&OsWindowFromHandleParams>) -> Result<OsWindow> {
        match params {
            None => match self.weak_windows.borrow().get(&hwnd) {
                None => failed("Handle refers to a foreign window, but params is None"),
                Some(weak) => match weak.upgrade() {
                    None => failed("Handle refers to a destroyed window"),
                    Some(strong) => Ok(OsWindow(strong)),
                },
            },
            Some(&OsWindowFromHandleParams {
                class_atom,
            }) => {
                let context = Rc::clone(&self.0);
                let os_window = OsSharedWindow {
                    context, hwnd, class_atom,
                };
                Ok(OsWindow(Rc::new(os_window)))
            },
        }
    }
}

impl OsWindow {
    pub fn handle(&self) -> WindowHandle {
        WindowHandle(self.hwnd)
    }
    pub fn set_title(&self, title: &str) -> Result<()> {
        let is_ok = unsafe {
            SetWindowTextW(self.hwnd, to_wide_with_nul(title).as_ptr())
        };
        if is_ok == FALSE {
            return winapi_fail("SetWindowTextW");
        }
        Ok(())
    }
    pub fn title(&self) -> Result<String> {
        let mut wide = [0; 1024];
        let nb_chars_without_nul = unsafe {
            GetWindowTextW(self.hwnd, wide.as_mut_ptr(), wide.len() as _)
        };
        if nb_chars_without_nul == 0 {
            return winapi_fail("GetWindowTextW");
        }
        assert!(nb_chars_without_nul < wide.len() as _);
        Ok(wide_string(&wide[..nb_chars_without_nul as usize]))
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
        let &WindowStyleHint {
            borders,
            title_bar_features,
        } = style_hint;
        unsafe {
            let mut style = GetWindowLongW(self.hwnd, GWL_STYLE) as u32;
            debug_assert_ne!(0, style); // This can't fail, it has no reason to
            if let Some(_) = borders {
                style |= WS_BORDER | WS_SIZEBOX;
            } else {
                style &= !(WS_BORDER | WS_SIZEBOX);
            };
            if let Some(TitleBarFeatures { minimize, maximize, close, }) = title_bar_features {
                if minimize {
                    style |= WS_MINIMIZEBOX;
                } else {
                    style &= !WS_MINIMIZEBOX;
                }
                if maximize {
                    style |= WS_MAXIMIZEBOX;
                } else {
                    style &= !WS_MAXIMIZEBOX;
                }
                self.set_close_button_enabled(close);
            } else {
                style &= !WS_CAPTION;
            }
            SetLastError(0); // See doc for SetWindowLongW()
            let previous = SetWindowLongW(self.hwnd, GWL_STYLE, style as _);
            let err = GetLastError();
            if previous == 0 && err != 0 {
                return winapi_fail_with_error_code("SetWindowLongW", err);
            }
        }
        Ok(())
    }
    // "How do I enable and disable the minimize, maximize, and close buttons in my caption bar?""
    // https://blogs.msdn.microsoft.com/oldnewthing/20100604-00/?p=13803
    fn set_close_button_enabled(&self, enabled: bool) {
        let flags = if enabled {
            MF_ENABLED
        } else {
            MF_DISABLED | MF_GRAYED
        };
        unsafe {
            EnableMenuItem(GetSystemMenu(self.hwnd, FALSE), SC_CLOSE as _, flags | MF_BYCOMMAND);
        }
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