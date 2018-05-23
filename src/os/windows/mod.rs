extern crate winapi;

pub mod hint;
pub use self::hint::set_hint;
pub mod context;
pub use self::context::{OsContext, OsSharedContext};
pub mod window;
pub use self::window::{OsWindow, OsWindowHandle, OsWindowFromHandleParams};
pub mod desktop;
pub mod cursor;
pub use self::cursor::OsCursor;
pub mod gl;
pub use self::gl::{OsGLContext, OsGLPixelFormat, OsGLProc};
pub mod event_instant;
pub use self::event_instant::OsEventInstant;
pub mod event;
pub mod device;
pub use self::device::{
    consts as device_consts,
    OsDeviceID, OsAxisInfo, OsDeviceInfo,
    controller::{OsControllerState, OsControllerInfo},
    keyboard::{OsKeyboardState, OsKeycode, OsKeysym},
    mouse::{OsMouseButtonsState},
    tablet::{OsTabletInfo, OsTabletPadButtonsState, OsTabletStylusButtonsState},
};


pub mod winapi_utils {
    pub use super::winapi::{
        shared::{windef::*, minwindef::*, ntdef::*},
        um::{winuser::*, libloaderapi::*, winbase::*, errhandlingapi::*},
    };
    pub use std::os::windows::ffi::{OsStringExt, OsStrExt};

    use std::slice;
    use std::ffi::{OsString, OsStr};
    use std::ptr;
    use error::{Result, failed};

    pub fn to_wide_without_nul(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().collect()
    }

    pub fn to_wide_with_nul(s: &str) -> Vec<u16> {
        let mut s = to_wide_without_nul(s);
        s.push(0);
        s
    }

    pub fn winapi_errorcode_string(err: DWORD) -> String {
        unsafe {
            let mut msg: *mut u16 = ptr::null_mut();
            let nb_chars_without_nul = FormatMessageW(
                FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
                ptr::null_mut(), err, MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT) as _,
                &mut msg as *mut *mut u16 as _, 0, ptr::null_mut()
            );

            let os_string = OsString::from_wide(slice::from_raw_parts(msg, nb_chars_without_nul as _));

            LocalFree(msg as _);

            os_string.to_string_lossy().into_owned().into()
        }
    }

    pub fn winapi_fail_with_error_code<T>(name: &str, err: DWORD) -> Result<T> {
        failed(format!("{}() failed with error {}: {}", name, err, winapi_errorcode_string(err)))
    }
    pub fn winapi_fail<T>(name: &str) -> Result<T> {
        let err = unsafe { GetLastError() };
        winapi_fail_with_error_code(name, err)
    }

    #[allow(non_snake_case)]
    pub fn MAKEINTATOM(atom: ATOM) -> *mut u16 {
        atom as WORD as usize as *mut _
    }
}