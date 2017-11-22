use os::*;
use gl::*;
use cursor::*;
use window::*;
use event::{PollIter, PeekIter, WaitIter};
use timeout::Timeout;
use std::rc::Rc;
use std::path::Path;
use Extent2;

#[derive(Debug)]
pub struct Context(pub(crate) OsContext);

/// Error types returned by this module.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum Error {
    Unsupported(Option<String>),
    Failed(String),
}


impl Context {
    /// Attempts to get one handle to the platform-specific display backend.
    /// 
    /// You should need only one.
    pub fn open() -> Result<Self, Error> {
        OsContext::open().map(Context)
    }

    /// X11-only specialization of `open()` where you can specify
    /// the name given to `XOpenDisplay()`.
    #[cfg(target_os="linux")] // FIXME: and BSDs too !
    pub fn open_x11_display_name(name: Option<&::std::ffi::CStr>) -> Result<Self, Error> {
        // NOTE: Keep full module path to CStr to prevent unused import in other platforms.
        OsContext::open_x11_display_name(name).map(Context)
    }

    /// Attempts to create a `Window` with the given settings.
    pub fn create_window(&mut self, settings: &WindowSettings) -> Result<Rc<Window>, Error> {
        let os_window = self.0.create_window(settings)?;
        let window = Rc::new(Window { os_window, fps_limit: None, });
        // TODO add to internal list
        Ok(window)
    }
    /// Same as `create_window()`, but immediately shows the window afterwards
    /// if it succeeds.
    pub fn create_window_and_show(&mut self, settings: &WindowSettings) -> Result<Rc<Window>, Error> {
        let w = self.create_window(settings)?;
        w.show();
        Ok(w)
    }

    pub fn poll_event_iter<'c>(&'c mut self) -> PollIter<'c> {
        PollIter { context: self }
    }
    pub fn peek_event_iter<'c>(&'c mut self) -> PeekIter<'c> {
        PeekIter { context: self }
    }
    pub fn wait_event_iter<'c>(&'c mut self, timeout: Timeout) -> WaitIter<'c> {
        WaitIter { context: self, timeout }
    }


    /// Attempts to retrieve the best pixel format for OpenGL-enabled windows
    /// and OpenGL contexts, given relevant settings.
    ///
    /// In the future, this might be improved by directly providing you
    /// with a list of candidates from which you can choose.
    pub fn choose_gl_pixel_format(&self, settings: &GLPixelFormatSettings)
        -> Result<GLPixelFormat, Error>
    {
        self.0.choose_gl_pixel_format(settings).map(GLPixelFormat)
    }

    /// Attempts to create a backend-specific OpenGL context.
    pub fn create_gl_context(&self, pf: &GLPixelFormat, cs: &GLContextSettings) -> Result<GLContext, Error> {
        self.0.create_gl_context(&pf.0, cs).map(GLContext)
    }
    /// Sames as `create_gl_context()`, but attempts to get a
    /// context that is not hardware-accelerated (on some platforms, this
    /// might try to load the Mesa driver).
    /// The use case for this is simple apps that don't specifically need a
    /// lot of perf, and would rather prefer saving battery power.
    /// 
    /// This won't attempt to fall back to the default implementation - in
    /// other words, this will succeed only if it is certain that there
    /// is a software implementation available AND a context could be created
    /// out of it.
    pub fn create_software_gl_context(&self, pf: &GLPixelFormat, cs: &GLContextSettings) -> Result<GLContext, Error> {
        self.0.create_software_gl_context(&pf.0, cs).map(GLContext)
    }

    /// Attempts to create an OpenGL context from a dynamically-loaded 
    /// library.
    pub fn create_gl_context_from_lib<P: AsRef<Path>>(&self, _pf: &GLPixelFormat, _cs: &GLContextSettings, _path: P) -> Result<GLContext, Error> {
        unimplemented!()
    }

    // Reply TRUE to WM_QUERYENDSESSION, in which case we then get WM_ENDSESSION.
    // ShutdownBlockReasonDestroy
    pub fn allow_session_termination(&mut self) -> Result<(), Error> {
        self.0.allow_session_termination()
    }
    // Reply FALSE to WM_QUERYENDSESSION.
    // ShutdownBlockReasonCreate
    pub fn disallow_session_termination(&mut self, reason: Option<String>) -> Result<(), Error> {
        self.0.disallow_session_termination(reason)
    }

    pub fn query_best_cursor_size(&self, _size_hint: Extent2<u32>) -> Extent2<u32> {
        unimplemented!{}
    }
    pub fn create_cursor(&self, _img: CursorFrame) -> Result<Cursor, Error> {
        unimplemented!{}
    }
    pub fn create_animated_cursor(&self, _anim: &[CursorFrame]) -> Result<Cursor, Error> {
        unimplemented!{}
    }
    pub fn create_system_cursor(&self, _s: SystemCursor) -> Result<Cursor, Error> {
        unimplemented!{}
    }
}

