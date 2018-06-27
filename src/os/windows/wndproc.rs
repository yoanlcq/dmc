use std::rc::Weak;
use super::OsSharedContext;
use super::winapi_utils as w32;
use self::w32::{
    HWND, UINT, WPARAM, LPARAM, LRESULT, DefWindowProcW,
    LOWORD, HIWORD, GET_X_LPARAM, GET_Y_LPARAM, RECT,
    WINDOWPOS, SWP_NOMOVE, SWP_NOSIZE,
};
use event::{Event, EventInstant};
use device::MouseButton;
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

pub extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // TODO: Reply to WM_GETMINMAXINFO: https://stackoverflow.com/a/22261818
    // TODO: Handle WM_MOVING. if !self.is_movable, restore window to initial position.
    let default_window_proc = || unsafe {
        DefWindowProcW(hwnd, msg, wparam, lparam)
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
        w32::WM_MOUSEACTIVATE
        | w32::WM_MOUSEHOVER
        | w32::WM_MOUSELEAVE
        | w32::WM_MOUSEMOVE
        | w32::WM_MOUSEWHEEL
        | w32::WM_MOUSEHWHEEL => {
            unimplemented!();
            default_window_proc()
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
                w32::WM_XBUTTONDBLCLK | w32::WM_XBUTTONDOWN | w32::WM_XBUTTONUP => unimplemented!(),
            };
            let mouse = unimplemented!();
            push_event(hwnd, Event::MouseMoved { mouse, position: Vec2::new(x as _, y as _) });
            push_event(hwnd, if is_down {
                Event::MouseButtonDown { mouse, button }
            } else {
                Event::MouseButtonUp { mouse, button }
            });
            0
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
        | w32::WM_SETFOCUS
        | w32::WM_KILLFOCUS
        | w32::WM_KEYDOWN
        | w32::WM_KEYUP
        | w32::WM_CHAR
        | w32::WM_DEADCHAR
        | w32::WM_UNICHAR
        | w32::WM_ACTIVATE
        | w32::WM_SETCURSOR
        | _ => default_window_proc(),
    }
}
