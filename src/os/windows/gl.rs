use std::os::raw::c_char;

#[derive(Debug)]
pub struct OsGLContext;
pub type OsGLPixelFormat = ();
pub type OsGLProc = ();

impl OsGLContext {
    pub unsafe fn get_proc_address(&self, name: *const c_char) -> Option<OsGLProc> {
        unimplemented!()
    }
}
