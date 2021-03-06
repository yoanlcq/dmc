use std::ptr;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::mem;
use error::{Result, failed};
use window::{Window, WindowSettings, WindowHandle, WindowStyleHint, WindowTypeHint, TitleBarFeatures, Borders};
use super::{OsContext, OsSharedContext, HCursor, winapi_utils::*};
use {Vec2, Extent2, Rect, Rgba};


pub type OsWindowHandle = HWND;

#[derive(Debug)]
pub struct OsWindowFromHandleParams {
    pub class_atom: ATOM,
    pub hicon: Option<HICON>,
    pub min_size: Option<Extent2<u32>>,
    pub max_size: Option<Extent2<u32>>,
    pub is_movable: bool,
}

#[derive(Debug)]
pub struct OsSharedWindow {
    pub context: Rc<OsSharedContext>,
    pub class_atom: ATOM,
    pub hwnd: HWND,
    pub own_dc: Result<HDC>,
    pub hicon: Cell<Option<HICON>>,
    pub min_size: Cell<Option<Extent2<u32>>>,
    pub max_size: Cell<Option<Extent2<u32>>>,
    pub is_movable: Cell<bool>,
    pub is_mouse_outside: Cell<bool>,
    pub cursor: RefCell<Rc<HCursor>>,
    pub is_cursor_visible: Cell<bool>,
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
            own_dc: _, // Destroyed with the window. DO NOT destroy it manually because it will fail.
            ref hicon,
            min_size: _, max_size: _, is_movable: _, is_mouse_outside: _,
            cursor: _, is_cursor_visible: _,
        } = self;

        match context.weak_windows.borrow_mut().remove(&hwnd) {
            Some(_weak) => trace!("Removed HWND {:?} from the context's list", hwnd),
            None => warn!("HWND {:?} is being destroyed but somehow wasn't in the context's list", hwnd),
        }

        unsafe {
            if let Some(hicon) = hicon.get() {
                DestroyIcon(hicon);
            }
            let is_ok = DestroyWindow(hwnd);
            let is_ok = UnregisterClassW(class_atom as _, context.hinstance());
        }
    }
}

impl OsContext {
    pub fn create_window(&self, settings: &WindowSettings) -> Result<OsWindow> {
        let &WindowSettings {
            ref opengl, high_dpi,
        } = settings;
        unsafe {
            let ex_style = WS_EX_ACCEPTFILES | WS_EX_OVERLAPPEDWINDOW;
            let style = WS_OVERLAPPEDWINDOW;
            let class_settings = super::context::ClassSettings {
                owndc: true, noclose: false,
            };
            if opengl.is_some() {
                assert!(class_settings.owndc);
            }
            let class_atom = self.get_or_register_class(&class_settings)?;
            let hwnd = CreateWindowExW(
                ex_style,
                MAKEINTATOM(class_atom),
                ptr::null(), // No title (yet)
                style,
                CW_USEDEFAULT, // x
                CW_USEDEFAULT, // y
                CW_USEDEFAULT, // w
                CW_USEDEFAULT, // h
                ptr::null_mut(), // No parent
                ptr::null_mut(), // No menu
                self.hinstance(),
                ptr::null_mut(), // No custom data pointer
            );
            if hwnd.is_null() {
                return winapi_fail("CreateWindowExW");
            }
            let own_dc = if !class_settings.owndc {
                failed("Window has no HDC; CS_OWNDC wasn't set in the class")
            } else {
                let own_dc = GetDC(hwnd);
                if own_dc.is_null() {
                    winapi_fail("GetDC() returned NULL")
                } else {
                    Ok(own_dc)
                }
            };

            let os_window = OsSharedWindow {
                context: Rc::clone(&self.0),
                class_atom,
                hwnd,
                own_dc,
                hicon: Cell::new(None),
                min_size: Cell::new(None),
                max_size: Cell::new(None),
                is_movable: Cell::new(true),
                is_mouse_outside: Cell::new(true), // XXX not correct?
                cursor: RefCell::new(self.create_default_system_cursor().unwrap().0),
                is_cursor_visible: Cell::new(true),
            };
            if let Some(opengl) = opengl.as_ref() {
                let pf = os_window.choose_gl_pixel_format(*opengl)?;
                os_window.set_pixel_format(&pf)?;
            }

            let _ = os_window.call_track_mouse_event();
            let os_window = Rc::new(os_window);
            self.weak_windows.borrow_mut().insert(hwnd, Rc::downgrade(&os_window));
            Ok(OsWindow(os_window))
        }
    }
    pub unsafe fn window_from_handle(&self, hwnd: OsWindowHandle, params: Option<&OsWindowFromHandleParams>) -> Result<OsWindow> {
        if IsWindow(hwnd) == FALSE {
            return failed("HWND doesn't refer to a window");
        }
        match params {
            None => match self.weak_windows.borrow().get(&hwnd) {
                None => failed("HWND refers to a foreign window, but params is None"),
                Some(weak) => match weak.upgrade() {
                    None => failed("HWND refers to a destroyed window"),
                    Some(strong) => Ok(OsWindow(strong)),
                },
            },
            Some(&OsWindowFromHandleParams {
                class_atom, hicon, min_size, max_size, is_movable,
            }) => {
                let os_window = OsSharedWindow {
                    context: Rc::clone(&self.0),
                    hwnd,
                    own_dc: {
                        let hdc = GetDC(hwnd);
                        if hdc.is_null() { winapi_fail("GetDC() returned NULL") } else { Ok(hdc) }
                    },
                    class_atom,
                    hicon: Cell::new(hicon),
                    min_size: Cell::new(min_size),
                    max_size: Cell::new(max_size),
                    is_movable: Cell::new(is_movable),
                    is_mouse_outside: Cell::new(true),
                    cursor: RefCell::new(self.create_default_system_cursor().unwrap().0),
                    is_cursor_visible: Cell::new(true),
                };
                let _ = os_window.call_track_mouse_event();
                Ok(OsWindow(Rc::new(os_window)))
            },
        }
    }
}

pub fn call_track_mouse_event(hwnd: HWND) -> Result<()> {
    let is_ok = unsafe {
        TrackMouseEvent(&mut TRACKMOUSEEVENT {
            cbSize: mem::size_of::<TRACKMOUSEEVENT>() as _,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 1, // or HOVER_DEFAULT which is ~400ms
        })
    };
    if is_ok == 0 {
        winapi_fail("TrackMouseEvent")
    } else {
        Ok(())
    }
}

impl OsSharedWindow {
    pub fn own_dc(&self) -> Result<HDC> {
        self.own_dc.clone()
    }
    pub fn handle(&self) -> WindowHandle {
        WindowHandle(self.hwnd)
    }
    pub fn call_track_mouse_event(&self) -> Result<()> {
        call_track_mouse_event(self.hwnd)
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
        // FIXME: use smallvec instead!
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
        if let Some(hicon) = self.hicon.get().take() {
            unsafe {
                DestroyIcon(hicon);
            }
        }
        // Convert to BGRA
        let mut data = data.to_vec();
        for pixel in &mut data {
            mem::swap(&mut pixel.r, &mut pixel.b);
        }
        let hicon = unsafe {
            let hicon = CreateIcon(self.context.hinstance(), size.w as _, size.h as _, 1, 32, ptr::null_mut(), data.as_ptr() as _);
            if hicon.is_null() {
                return winapi_fail("CreateIcon");
            }
            SendMessageW(self.hwnd, WM_SETICON, ICON_SMALL as _, hicon as _);
            SendMessageW(self.hwnd, WM_SETICON, ICON_BIG as _, hicon as _);
            hicon
        };
        self.hicon.set(Some(hicon));
        Ok(())
    }
    pub fn icon(&self) -> Result<(Extent2<u32>, Vec<Rgba<u8>>)> {
        /* Complicated
        unsafe {
            let hicon: HICON = DefWindowProcW(self.hwnd, WM_GETICON, ICON_BIG as _, 96) as _; // XXX dumb DPI value
            let mut iconinfo = mem::zeroed();
            let is_ok = GetIconInfo(hicon, &mut iconinfo);
            let hbitmap = iconinfo.hbmColor;
            let bitmapinfo = mem::zeroed();
            let status = GetDIBits(hdc, hbitmap, 0, nb_scan_lines, bits.as_mut_ptr(), &mut bitmapinfo, DIB_RGB_COLORS);
            if status == 0 || status == ERROR_INVALID_PARAMETER {
                Err()?;
            }
        }
        */
        unimplemented!()
    }
    pub fn reset_icon(&self) -> Result<()> {
        unsafe {
            if let Some(hicon) = self.hicon.get().take() {
                DestroyIcon(hicon);
            }
            SendMessageW(self.hwnd, WM_SETICON, ICON_SMALL as _, 0);
            SendMessageW(self.hwnd, WM_SETICON, ICON_BIG as _, 0);
        }
        Ok(())
    }
    pub fn set_type_hint(&self, type_hint: &WindowTypeHint) -> Result<()> {
        if type_hint.net_wm.is_empty() {
            return Ok(());
        }
        // FIXME: It's complicated. This involves making other window classes and setting window style flags.
        use ::window::NetWMWindowType;
        match type_hint.net_wm[0] {
            NetWMWindowType::Normal => (),
            NetWMWindowType::Desktop => (),
            NetWMWindowType::Dock => (),
            NetWMWindowType::Toolbar => (),
            NetWMWindowType::Menu => (),
            NetWMWindowType::Utility => (),
            NetWMWindowType::Splash => (),
            NetWMWindowType::Dialog => (),
            NetWMWindowType::DropdownMenu => (),
            NetWMWindowType::PopupMenu => (),
            NetWMWindowType::Tooltip => (),
            NetWMWindowType::Notification => (),
            NetWMWindowType::Combo => (),
            NetWMWindowType::DND => (),
        };
        Ok(())
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
            self.set_window_long_ptr(GWL_STYLE, style as _)
        }
    }
    fn set_window_long_ptr(&self, gwl: i32, val: isize) -> Result<()> {
        unsafe {
            SetLastError(0); // See doc for SetWindowLongW()
            let previous = SetWindowLongPtrW(self.hwnd, gwl, val as _); // val is i32, not isize, on 32-bit windows. Urgh.
            let err = GetLastError();
            if previous == 0 && err != 0 {
                return winapi_fail_with_error_code("SetWindowLongPtrW", err);
            }
        }
        self.set_window_pos(Default::default(), SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED)
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
        unsafe {
            let is_ok = BringWindowToTop(self.hwnd);
            if is_ok == FALSE {
                return winapi_fail("BringWindowToTop");
            }
        }
        Ok(())
    }
    pub fn set_min_size(&self, size: Extent2<u32>) -> Result<()> {
        self.min_size.set(Some(size));
        Ok(())
    }
    pub fn set_max_size(&self, size: Extent2<u32>) -> Result<()> {
        self.max_size.set(Some(size));
        Ok(())
    }

    pub fn set_resizable(&self, resizable: bool) -> Result<()> {
        let mut style = unsafe {
            GetWindowLongW(self.hwnd, GWL_STYLE) as u32
        };
        if resizable {
            style |= WS_SIZEBOX;
        } else {
            style &= !WS_SIZEBOX;
        }
        self.set_window_long_ptr(GWL_STYLE, style as _)
    }
    pub fn is_resizable(&self) -> Result<bool> {
        unsafe {
            Ok((GetWindowLongW(self.hwnd, GWL_STYLE) as u32 & WS_SIZEBOX) != 0)
        }
    }
    pub fn set_movable(&self, movable: bool) -> Result<()> {
        self.is_movable.set(movable);
        Ok(())
    }
    pub fn is_movable(&self) -> Result<bool> {
        Ok(self.is_movable.get())
    }
    fn show_window(&self, show_cmd: i32) -> Result<()> {
        unsafe {
            // No error to handle here!
            ShowWindow(self.hwnd, show_cmd);
        }
        Ok(())
    }
    fn show_cmd(&self) -> Result<u32> {
        unsafe {
            let mut windowplacement = WINDOWPLACEMENT {
                length: mem::size_of::<WINDOWPLACEMENT>() as _,
                .. mem::zeroed()
            };
            let is_ok = GetWindowPlacement(self.hwnd, &mut windowplacement);
            if is_ok == FALSE {
                return winapi_fail("GetWindowPlacement");
            }
            Ok(windowplacement.showCmd)
        }
    }
    pub fn show(&self) -> Result<()> {
        self.show_window(SW_SHOW)
    }
    pub fn hide(&self) -> Result<()> {
        self.show_window(SW_HIDE)
    }
    pub fn is_visible(&self) -> Result<bool> {
        Ok(self.show_cmd()? as i32 != SW_HIDE)
    }
    pub fn toggle_visibility(&self) -> Result<()> {
        if self.is_visible()? {
            self.hide()
        } else {
            self.show()
        }
    }
    pub fn minimize(&self) -> Result<()> {
        self.show_window(SW_MINIMIZE)
    }
    pub fn unminimize(&self) -> Result<()> {
        self.show_window(SW_RESTORE)
    }
    pub fn is_minimized(&self) -> Result<bool> {
        Ok(self.show_cmd()? as i32 == SW_MINIMIZE)
    }
    pub fn toggle_minimize(&self) -> Result<()> {
        if self.is_minimized()? {
            self.unminimize()
        } else {
            self.minimize()
        }
    }

    pub fn maximize(&self) -> Result<()> {
        self.show_window(SW_MAXIMIZE)
    }
    pub fn unmaximize(&self) -> Result<()> {
        self.show_window(SW_RESTORE)
    }
    pub fn is_maximized(&self) -> Result<bool> {
        Ok(self.show_cmd()? as i32 == SW_MAXIMIZE)
    }
    pub fn toggle_maximize(&self) -> Result<()> {
        if self.is_maximized()? {
            self.unmaximize()
        } else {
            self.maximize()
        }
    }


    // Urgh, these are complicated
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
    fn flash_window_ex(&self, flags: DWORD, ucount: u32) -> Result<()> {
        unsafe {
            let mut flashwinfo = FLASHWINFO {
                cbSize: mem::size_of::<FLASHWINFO>() as _,
                hwnd: self.hwnd,
                dwFlags: flags,
                uCount: ucount,
                dwTimeout: 0, // Use default
            };
            FlashWindowEx(&mut flashwinfo);
        }
        Ok(())
    }
    pub fn demand_attention(&self) -> Result<()> {
        self.flash_window_ex(FLASHW_ALL, 3)
    }
    pub fn demand_urgent_attention(&self) -> Result<()> {
        self.flash_window_ex(FLASHW_ALL | FLASHW_TIMERNOFG, 0xffffffff)
    }
    pub fn position_and_size(&self) -> Result<Rect<i32, u32>> {
        unsafe {
            let mut r: RECT = mem::zeroed();
            let is_ok = GetWindowRect(self.hwnd, &mut r);
            if is_ok == FALSE {
                return winapi_fail("GetWindowRect");
            }
            let r = Rect {
                x: r.left,
                y: r.top,
                w: (r.right + 1 - r.left) as _,
                h: (r.bottom + 1 - r.top) as _,
            };
            Ok(r)
        }
    }
    pub fn position(&self) -> Result<Vec2<i32>> {
        self.position_and_size().map(|ps| ps.position())
    }
    pub fn size(&self) -> Result<Extent2<u32>> {
        self.position_and_size().map(|ps| ps.extent())
    }
    pub fn canvas_size(&self) -> Result<Extent2<u32>> {
        self.size() // FIXME: not true!
    }

    fn set_window_pos(&self, r: Rect<i32, u32>, flags: u32) -> Result<()> {
        unsafe {
            let is_ok = SetWindowPos(self.hwnd, ptr::null_mut(), r.x, r.y, r.w as _, r.h as _, flags);
            if is_ok == FALSE {
                return winapi_fail("SetWindowPos");
            }
        }
        Ok(())
    }
    pub fn set_position(&self, pos: Vec2<i32>) -> Result<()> {
        self.set_window_pos(Rect { x: pos.x, y: pos.y, .. Default::default() }, SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED)
    }
    pub fn set_size(&self, size: Extent2<u32>) -> Result<()> {
        self.set_window_pos(Rect { w: size.w, h: size.h, .. Default::default() }, SWP_NOMOVE | SWP_NOZORDER | SWP_FRAMECHANGED)
    }
    pub fn set_position_and_size(&self, r: Rect<i32, u32>) -> Result<()> {
        self.set_window_pos(r, SWP_NOZORDER | SWP_FRAMECHANGED)
    }
    // See WS_EX_LAYERED and UpdateLayeredWindow ()
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
    // SetCursorPos
    pub fn set_mouse_position(&self, pos: Vec2<i32>) -> Result<()> {
        unimplemented!()
    }
    // GetCursorPos
    pub fn mouse_position(&self) -> Result<Vec2<i32>> {
        unimplemented!()
    }
    pub fn trap_mouse(&self) -> Result<()> {
        unimplemented!()
    }
    // TODO: Use RedrawWindow()
    pub fn clear(&self) -> Result<()> {
        unimplemented!()
    }
    pub fn clear_rect(&self, r: Rect<i32, u32>) -> Result<()> {
        unimplemented!()
    }
}
