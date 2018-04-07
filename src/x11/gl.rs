use std::os::raw::c_char;

pub type X11GLProc = unsafe extern "C" fn();

#[derive(Debug)]
pub struct X11GLContext;
#[derive(Debug)]
pub struct X11GLPixelFormat;

impl X11GLContext {
    pub unsafe fn get_proc_address(&self, _name: *const c_char) -> Option<X11GLProc> {
        unimplemented!{}
    }
}
