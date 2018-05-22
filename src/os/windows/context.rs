use std::cell::RefCell;
use std::mem;
use std::ptr;
use std::collections::HashMap;
use super::winapi_utils::*;
use error::Result;

extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unimplemented!()
}


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ClassSettings {
    pub owndc: bool,
    pub noclose: bool,
}

#[derive(Debug)]
pub struct OsContext {
    hinstance: HINSTANCE,
    class_atoms: RefCell<HashMap<ClassSettings, ATOM>>,
}

impl Drop for OsContext {
    fn drop(&mut self) {
        let &mut Self {
            hinstance, ref class_atoms,
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

impl OsContext {
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
            hCursor: ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
        };
        let class_atom = unsafe {
            RegisterClassExW(&wclass)
        };
        if class_atom == 0 {
            return winapi_fail("RegisterClassExW");
        }
        Ok(class_atom)
    }
}

impl OsContext {
    pub fn new() -> Result<Self> {
        let c = unsafe {
            OsContext {
                hinstance: GetModuleHandleW(ptr::null()),
                class_atoms: RefCell::new(HashMap::new()),
            }
        };
        Ok(c)
    }
    pub fn untrap_mouse(&self) -> Result<()> {
        unimplemented!()
    }
}

