use std::cell::RefCell;
use std::mem;
use std::ptr;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::ops::Deref;
use super::{winapi_utils::*, OsSharedWindow};
use error::Result;

extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // TODO: Reply to WM_GETMINMAXINFO: https://stackoverflow.com/a/22261818
    // TODO: Handle WM_MOVING. if !self.is_movable, restore window to initial position.
    unimplemented!()
}


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ClassSettings {
    pub owndc: bool,
    pub noclose: bool,
}

#[derive(Debug)]
pub struct OsSharedContext {
    hinstance: HINSTANCE,
    class_atoms: RefCell<HashMap<ClassSettings, ATOM>>,
    pub weak_windows: RefCell<HashMap<HWND, Weak<OsSharedWindow>>>,
}
#[derive(Debug)]
pub struct OsContext(pub(crate) Rc<OsSharedContext>);

impl Deref for OsContext {
    type Target = OsSharedContext;
    fn deref(&self) -> &OsSharedContext {
        &self.0
    }
}

impl Drop for OsSharedContext {
    fn drop(&mut self) {
        let &mut Self {
            hinstance, ref class_atoms, ref weak_windows,
        } = self;
        unsafe {
            for class_atom in class_atoms.borrow().values() {
                UnregisterClassW(MAKEINTATOM(*class_atom), hinstance);
            }
        }
    }
}

mod classname_token {
    use ::std::mem;
    use ::std::num::Wrapping;

    // This is a global because class names are process-wide,
    // and nobody said we couldn't have multiple contexts.
    static mut CLASSNAME_TOKEN: Wrapping<u32> = Wrapping(0);

    pub fn get_new_unique() -> u32 {
        let cur = unsafe { &mut CLASSNAME_TOKEN };
        let next = *cur + Wrapping(1);
        mem::replace(cur, next).0
    }
}

impl OsSharedContext {
    pub fn hinstance(&self) -> HINSTANCE {
        self.hinstance
    }
    pub fn get_or_register_class(&self, settings: &ClassSettings) -> Result<ATOM> {
        if let Some(atom) = self.class_atoms.borrow().get(settings) {
            return Ok(*atom);
        }

        let &ClassSettings {
            owndc, noclose,
        } = settings;

        let classname = to_wide_with_nul(&format!("DMC WNDCLASS {}", classname_token::get_new_unique()));
        assert!(classname.len() < 256);
        
        let wclass = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as _,
            hInstance: self.hinstance(),
            lpfnWndProc: Some(wndproc),
            lpszClassName: classname.as_ptr(),
            style: CS_DBLCLKS | CS_HREDRAW | CS_VREDRAW | (CS_OWNDC * owndc as u32) | (CS_NOCLOSE * noclose as u32),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hIcon: ptr::null_mut(),
            hIconSm: ptr::null_mut(),
            hCursor: unsafe { LoadCursorW(ptr::null_mut(), IDC_ARROW) },
            hbrBackground: unsafe { GetStockObject(WHITE_BRUSH as _) as _ }, // XXX might cause trouble??
            lpszMenuName: ptr::null(),
        };
        let class_atom = unsafe {
            RegisterClassExW(&wclass)
        };
        if class_atom == 0 {
            return winapi_fail("RegisterClassExW");
        }
        let previous = self.class_atoms.borrow_mut().insert(*settings, class_atom);
        assert!(previous.is_none()); // Must have been checked in early return
        Ok(class_atom)
    }
}

mod dpi_awareness {
    use super::*;

    static mut IS_SET: Option<Result<()>> = None;

    pub fn init_once() {
        unsafe {
            // FIXME: This is all wrong! This should involve dynamic loading of DLLs so it works on
            // any Windows version, and it's a bit more complicated than that.
            if IS_SET.is_some() {
                return;
            }
            let hresult = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
            IS_SET = Some(hresult_to_result("SetProcessDpiAwareness", hresult));
        }
    }
}

impl OsSharedContext {
    fn new() -> Result<Self> {
        dpi_awareness::init_once();
        let c = unsafe {
            Self {
                hinstance: GetModuleHandleW(ptr::null()),
                class_atoms: RefCell::new(HashMap::new()),
                weak_windows: RefCell::new(HashMap::new()),
            }
        };
        Ok(c)
    }
}

impl OsContext {
    pub fn new() -> Result<Self> {
        Ok(OsContext(Rc::new(OsSharedContext::new()?)))
    }
    pub fn untrap_mouse(&self) -> Result<()> {
        unimplemented!()
    }
}

