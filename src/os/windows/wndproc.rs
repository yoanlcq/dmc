use std::rc::{Rc, Weak};
use std::time::Instant;
use super::{OsSharedContext, OsSharedWindow, OsDeviceID, OsEventInstant};
use super::winapi_utils as w32;
use self::w32::{
    HWND, UINT, WPARAM, LPARAM, LRESULT, DefWindowProcW,
    LOWORD, HIWORD, GET_X_LPARAM, GET_Y_LPARAM, GET_XBUTTON_WPARAM,
    RECT, POINT,
    WINDOWPOS, SWP_NOMOVE, SWP_NOSIZE,
    ClientToScreen,
};
use event::{Event, EventInstant};
use device::{DeviceID, MouseButton, Key, Keycode};
use window::WindowHandle;
use {Vec2, Extent2};

pub static mut CONTEXT: Option<Weak<OsSharedContext>> = None;

fn push_event(hwnd: HWND, ev: Event) {
    // hwnd might be used later to retrieve the Context via a global list of windows (I don't like this :/ )
    // Avoid panicking, because we might receive messages even though we have no context current. Windows can do whatever it wants with window procs.
    let weak = match unsafe { CONTEXT.as_ref() } {
        None => return,
        Some(weak) => weak,
    };
    let context = match weak.upgrade() {
        None => return,
        Some(strong) => strong,
    };
    context.push_event(ev);
}

fn retrieve_window(hwnd: HWND) -> Option<Rc<OsSharedWindow>> {
    let weak = match unsafe { CONTEXT.as_ref() } {
        None => return None,
        Some(weak) => weak,
    };
    let context = match weak.upgrade() {
        None => return None,
        Some(strong) => strong,
    };
    let out = { // Urgh thanks borrow checker
        context.weak_windows.borrow().get(&hwnd).map(Weak::upgrade).unwrap_or(None)
    };
    out
}

pub extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // TODO: Reply to WM_GETMINMAXINFO: https://stackoverflow.com/a/22261818
    // TODO: Handle WM_MOVING. if !self.is_movable, restore window to initial position.
    let default_window_proc = || unsafe {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    };
    let get_root_position = |x, y| {
        let mut point = POINT { x, y };
        let is_ok = unsafe {
            ClientToScreen(hwnd, &mut point)
        };
        Vec2::new(point.x as f64, point.y as f64)
    };

    match msg {
        // This message is actually never received by windows, because only GetMessage() and PeekMessage() functions retrieve it. But oh well.
        w32::WM_QUIT => {
            push_event(hwnd, Event::Quit);
            0
        },
        w32::WM_CLOSE => {
            push_event(hwnd, Event::WindowCloseRequested { window: WindowHandle(hwnd) });
            0
        },
        // Sent after a window has been moved.
        w32::WM_MOVE => {
            let x = LOWORD(lparam as _) as i16;
            let y = HIWORD(lparam as _) as i16;
            push_event(hwnd, Event::WindowMoved { window: WindowHandle(hwnd), position: Vec2::new(x as _, y as _), by_user: false });
            0
        },
        w32::WM_MOVING => {
            let rect = unsafe {
                &mut *(lparam as *mut RECT)
            };
            // NOTE: We are allowed to mutate the rect
            push_event(hwnd, Event::WindowMoved { window: WindowHandle(hwnd), position: Vec2::new(rect.left as _, rect.top as _), by_user: false });
            1
        },
        w32::WM_SIZE => {
            let w = LOWORD(lparam as _) as i16;
            let h = HIWORD(lparam as _) as i16;
            push_event(hwnd, Event::WindowResized { window: WindowHandle(hwnd), size: Extent2::new(w as _, h as _), by_user: false });
            0
        },
        w32::WM_SIZING => {
            let rect = unsafe {
                &mut *(lparam as *mut RECT)
            };
            // NOTE: We are allowed to mutate the rect
            push_event(hwnd, Event::WindowResized { window: WindowHandle(hwnd), size: Extent2::new((rect.right - rect.left) as _, (rect.bottom - rect.top) as _), by_user: false });
            1
        },
        // Sent to a window whose size, position, or place in the Z order has changed as a result of a call to the SetWindowPos function or another window-management function.
        w32::WM_WINDOWPOSCHANGED => {
            let wpos = unsafe {
                &*(lparam as *const WINDOWPOS)
            };
            if (wpos.flags & SWP_NOMOVE) == 0 {
                push_event(hwnd, Event::WindowMoved { window: WindowHandle(hwnd), position: Vec2::new(wpos.x as _, wpos.y as _), by_user: true });
            }
            if (wpos.flags & SWP_NOSIZE) == 0 {
                push_event(hwnd, Event::WindowResized { window: WindowHandle(hwnd), size: Extent2::new(wpos.cx as _, wpos.cy as _), by_user: true });
            }
            0
        },
        // Sent to a window whose size, position, or place in the Z order is about to change as a result of a call to the SetWindowPos function or another window-management function.
        w32::WM_WINDOWPOSCHANGING => {
            let wpos = unsafe {
                &*(lparam as *const WINDOWPOS)
            };
            if (wpos.flags & SWP_NOMOVE) == 0 {
                push_event(hwnd, Event::WindowMoved { window: WindowHandle(hwnd), position: Vec2::new(wpos.x as _, wpos.y as _), by_user: true });
            }
            if (wpos.flags & SWP_NOSIZE) == 0 {
                push_event(hwnd, Event::WindowResized { window: WindowHandle(hwnd), size: Extent2::new(wpos.cx as _, wpos.cy as _), by_user: true });
            }
            0
        },
        //Sent when the cursor is in an inactive window and the user presses a mouse button
        w32::WM_MOUSEACTIVATE => default_window_proc(),
        w32::WM_MOUSEHOVER => {
            let _ = super::window::call_track_mouse_event(hwnd);
            default_window_proc()
            /*
            let x = GET_X_LPARAM(lparam); 
            let y = GET_Y_LPARAM(lparam); 
            let root_position = get_root_position(x, y);
            let mouse = DeviceID(OsDeviceID::MainMouse);
            let instant = EventInstant(OsEventInstant::Wndproc(Instant::now()));
            let window = WindowHandle(hwnd);
            push_event(hwnd, Event::MouseEnter { mouse, instant, window, is_focused: false, is_grabbed: false, });
            push_event(hwnd, Event::MouseMotion { mouse, window, instant, position: Vec2::new(x as _, y as _), root_position });
            let _ = super::window::call_track_mouse_event(hwnd);
            0
            */
        },
        w32::WM_MOUSELEAVE => {
            // lparam and wparam are not used and must be zero
            let mouse = DeviceID(OsDeviceID::MainMouse);
            let instant = EventInstant(OsEventInstant::Wndproc(Instant::now()));
            let window = WindowHandle(hwnd);
            push_event(hwnd, Event::MouseLeave { mouse, instant, window, was_focused: false, was_grabbed: false, });
            retrieve_window(hwnd).unwrap().is_mouse_outside.set(true);
            // DO NOT call
            // let _ = super::window::call_track_mouse_event(hwnd);
            0
        },
        w32::WM_MOUSEMOVE => {
            let x = GET_X_LPARAM(lparam);
            let y = GET_Y_LPARAM(lparam);
            let mouse = DeviceID(OsDeviceID::MainMouse);
            let window = WindowHandle(hwnd);
            let instant = EventInstant(OsEventInstant::Wndproc(Instant::now()));
            let root_position = get_root_position(x, y);
            if retrieve_window(hwnd).unwrap().is_mouse_outside.replace(false) {
                push_event(hwnd, Event::MouseEnter { mouse, instant, window, is_focused: false, is_grabbed: false, });
                let _ = super::window::call_track_mouse_event(hwnd);
            }
            push_event(hwnd, Event::MouseMotion { mouse, window, instant, position: Vec2::new(x as _, y as _), root_position });
            0
        },
        w32::WM_MOUSEWHEEL
        | w32::WM_MOUSEHWHEEL => {
            let x = GET_X_LPARAM(lparam);
            let y = GET_Y_LPARAM(lparam);
            let delta = w32::GET_WHEEL_DELTA_WPARAM(wparam) as f64 / w32::WHEEL_DELTA as f64;
            let scroll = match msg {
                w32::WM_MOUSEWHEEL => Vec2::new(0., delta),
                w32::WM_MOUSEHWHEEL => Vec2::new(delta, 0.),
                _ => unreachable!(),
            };
            let mouse = DeviceID(OsDeviceID::MainMouse);
            let window = WindowHandle(hwnd);
            let instant = EventInstant(OsEventInstant::Wndproc(Instant::now()));
            let root_position = get_root_position(x, y);
            push_event(hwnd, Event::MouseMotion { mouse, window, instant, position: Vec2::new(x as _, y as _), root_position });
            push_event(hwnd, Event::MouseScroll { mouse, window, instant, scroll });
            0
        },
        w32::WM_LBUTTONDBLCLK
        | w32::WM_LBUTTONDOWN
        | w32::WM_LBUTTONUP
        | w32::WM_MBUTTONDBLCLK
        | w32::WM_MBUTTONDOWN
        | w32::WM_MBUTTONUP
        | w32::WM_RBUTTONDBLCLK
        | w32::WM_RBUTTONDOWN
        | w32::WM_RBUTTONUP
        | w32::WM_XBUTTONDBLCLK
        | w32::WM_XBUTTONDOWN
        | w32::WM_XBUTTONUP => {
            let x = GET_X_LPARAM(lparam);
            let y = GET_Y_LPARAM(lparam);
            let is_down = match msg {
                  w32::WM_LBUTTONUP | w32::WM_MBUTTONUP | w32::WM_RBUTTONUP | w32::WM_XBUTTONUP => false,
                _ => true,
            };
            let button = match msg {
                w32::WM_LBUTTONDBLCLK | w32::WM_LBUTTONDOWN | w32::WM_LBUTTONUP => MouseButton::Left,
                w32::WM_MBUTTONDBLCLK | w32::WM_MBUTTONDOWN | w32::WM_MBUTTONUP => MouseButton::Middle,
                w32::WM_RBUTTONDBLCLK | w32::WM_RBUTTONDOWN | w32::WM_RBUTTONUP => MouseButton::Right,
                w32::WM_XBUTTONDBLCLK | w32::WM_XBUTTONDOWN | w32::WM_XBUTTONUP => match GET_XBUTTON_WPARAM(wparam) {
                    w32::XBUTTON1 => ::device::mouse::XBUTTON1,
                    w32::XBUTTON2 => ::device::mouse::XBUTTON2,
                    other => MouseButton::Other(other as _),
                },
                _ => unreachable!(),
            };
            let clicks = match msg {
                w32::WM_LBUTTONDBLCLK | w32::WM_MBUTTONDBLCLK | w32::WM_RBUTTONDBLCLK | w32::WM_XBUTTONDBLCLK => Some(2),
                _ => None,
            };
            let mouse = DeviceID(OsDeviceID::MainMouse);
            let instant = EventInstant(OsEventInstant::Wndproc(Instant::now()));
            let root_position = get_root_position(x, y);

            let window = WindowHandle(hwnd);
            push_event(hwnd, Event::MouseMotion { mouse, window, instant, position: Vec2::new(x as _, y as _), root_position });
            push_event(hwnd, if is_down {
                Event::MouseButtonPressed { mouse, window, instant, button, clicks, }
            } else {
                Event::MouseButtonReleased { mouse, window, instant, button }
            });
            match msg {
                w32::WM_XBUTTONDBLCLK | w32::WM_XBUTTONDOWN | w32::WM_XBUTTONUP => 1,
                _ => 0,
            }
        },
        w32::WM_SETFOCUS | w32::WM_KILLFOCUS => {
            let keyboard = DeviceID(OsDeviceID::MainKeyboard);
            let window = WindowHandle(hwnd);
            push_event(hwnd, match msg {
                w32::WM_SETFOCUS => Event::KeyboardFocusGained { keyboard, window },
                w32::WM_KILLFOCUS => Event::KeyboardFocusLost { keyboard, window },
                _ => unreachable!(),
            });
            0
        },
        w32::WM_KEYDOWN | w32::WM_KEYUP => {
            let vkey = wparam;
            let repeat_count = lparam & 0xffff;
            let scan_code = (lparam >> 16) & 0xff;
            let is_repeat = ((lparam >> 30) & 1) != 0;
            let keyboard = DeviceID(OsDeviceID::MainKeyboard);
            let window = WindowHandle(hwnd);
            let instant = EventInstant(OsEventInstant::Wndproc(Instant::now()));
            let key = Key {
                code: Keycode(scan_code as _),
                sym: super::device::keyboard::keysym_from_vkey(vkey as _).into(),
            };
            push_event(hwnd, match msg {
                w32::WM_KEYDOWN => Event::KeyboardKeyPressed { keyboard, window, instant, key, is_repeat, repeat_count: repeat_count as _, },
                w32::WM_KEYUP => Event::KeyboardKeyReleased { keyboard, window, instant, key },
                _ => unreachable!(),
            });
            0
        },
        w32::WM_DEADCHAR => default_window_proc(),
        w32::WM_CHAR | w32::WM_UNICHAR => if msg == w32::WM_UNICHAR && wparam == w32::UNICODE_NOCHAR {
            1
        } else {
            let repeat_count = lparam & 0xffff;
            let is_repeat = ((lparam >> 30) & 1) != 0;
            let keyboard = DeviceID(OsDeviceID::MainKeyboard);
            let window = WindowHandle(hwnd);
            let instant = EventInstant(OsEventInstant::Wndproc(Instant::now()));
            if let Some(char) = ::std::char::from_u32(wparam as _) {
                push_event(hwnd, Event::KeyboardTextChar { keyboard, window, instant, char, is_repeat, repeat_count: repeat_count as _ });
            }
            0
        },
        w32::WM_SETCURSOR => {
            if w32::LOWORD(lparam as _) as isize == w32::HTCLIENT {
                let window = retrieve_window(hwnd).unwrap();
                unsafe { w32::ShowCursor(window.is_cursor_visible.get() as _); }
                let cursor = window.cursor.borrow();
                unsafe { w32::SetCursor(cursor.0); }
                1
            } else {
                default_window_proc()
            }
        },
        w32::WM_ACTIVATEAPP
        | w32::WM_CANCELMODE
        | w32::WM_CHILDACTIVATE
        | w32::WM_COMPACTING
        | w32::WM_CREATE
        | w32::WM_DESTROY
        | w32::WM_DPICHANGED
        | w32::WM_ENABLE
        | w32::WM_ENTERSIZEMOVE
        | w32::WM_EXITSIZEMOVE
        | w32::WM_GETICON
        | w32::WM_GETMINMAXINFO
        | w32::WM_INPUTLANGCHANGE
        | w32::WM_INPUTLANGCHANGEREQUEST
        | w32::WM_NCACTIVATE
        | w32::WM_NCCALCSIZE
        | w32::WM_NCCREATE
        | w32::WM_NCDESTROY
        | w32::WM_NULL
        | w32::WM_QUERYDRAGICON
        | w32::WM_QUERYOPEN
        | w32::WM_SHOWWINDOW
        | w32::WM_STYLECHANGED
        | w32::WM_STYLECHANGING
        | w32::WM_THEMECHANGED
        | w32::WM_USERCHANGED
        | w32::WM_CAPTURECHANGED
        | w32::WM_NCHITTEST
        | w32::WM_NCLBUTTONDBLCLK
        | w32::WM_NCLBUTTONDOWN
        | w32::WM_NCLBUTTONUP
        | w32::WM_NCMBUTTONDBLCLK
        | w32::WM_NCMBUTTONDOWN
        | w32::WM_NCMBUTTONUP
        | w32::WM_NCMOUSEHOVER
        | w32::WM_NCMOUSELEAVE
        | w32::WM_NCMOUSEMOVE
        | w32::WM_NCRBUTTONDBLCLK
        | w32::WM_NCRBUTTONDOWN
        | w32::WM_NCRBUTTONUP
        | w32::WM_NCXBUTTONDBLCLK
        | w32::WM_NCXBUTTONDOWN
        | w32::WM_NCXBUTTONUP
        | w32::WM_ACTIVATE
        | _ => default_window_proc(),
    }
}
