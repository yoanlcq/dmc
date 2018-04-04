use std::time::Duration;
use std::rc::Rc;
use std::path::Path;
use os::*;
use gl::*;
use cursor::*;
use window::*;
use event::{PollIter, WaitIter};
use Extent2;
use error::Result;

#[derive(Debug)]
pub struct Context(pub(crate) OsContext);

impl !Send for Context {}
impl !Sync for Context {}

impl Context {
    /// Attempts to get one handle to the platform-specific display backend.
    /// 
    /// You should need only one.
    pub fn new() -> Result<Self> {
        OsContext::new().map(Context)
    }

    /// X11-only specialization of `open()` where you can specify
    /// the name given to `XOpenDisplay()`.
    #[cfg(any(target_os="linux", target_os="freebsd", target_os="dragonfly", target_os="openbsd", target_os="netbsd"))]
    pub fn with_x11_display_name(name: Option<&::std::ffi::CStr>) -> Result<Self> {
        OsContext::with_x11_display_name(name).map(Context)
    }

    /// Attempts to create a `Window` with the given settings.
    pub fn create_window(&mut self, settings: &WindowSettings) -> Result<Rc<Window>> {
        let os_window = self.0.create_window(settings)?;
        let window = Rc::new(Window { os_window, fps_limit: None, });
        self.0.add_weak_window(&window);
        Ok(window)
    }
    /// Same as `create_window()`, but immediately shows the window afterwards
    /// if it succeeds.
    pub fn create_window_and_show(&mut self, settings: &WindowSettings) -> Result<Rc<Window>> {
        let w = self.create_window(settings)?;
        w.show()?;
        Ok(w)
    }

    pub fn events_poll_iter<'c>(&'c mut self) -> PollIter<'c> {
        PollIter { context: self }
    }
    pub fn events_wait_iter<'c>(&'c mut self, timeout: Duration) -> WaitIter<'c> {
        WaitIter { context: self, timeout }
    }


    /// Attempts to retrieve the best pixel format for OpenGL-enabled windows
    /// and OpenGL contexts, given relevant settings.
    ///
    /// In the future, this might be improved by directly providing you
    /// with a list of candidates from which you can choose.
    pub fn choose_gl_pixel_format(&self, settings: &GLPixelFormatSettings)
        -> Result<GLPixelFormat>
    {
        self.0.choose_gl_pixel_format(settings).map(GLPixelFormat)
    }

    /// Attempts to create a backend-specific OpenGL context.
    pub fn create_gl_context(&self, pf: &GLPixelFormat, cs: &GLContextSettings) -> Result<GLContext> {
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
    pub fn create_software_gl_context(&self, pf: &GLPixelFormat, cs: &GLContextSettings) -> Result<GLContext> {
        self.0.create_software_gl_context(&pf.0, cs).map(GLContext)
    }

    /// Attempts to create an OpenGL context from a dynamically-loaded 
    /// library.
    pub fn create_gl_context_from_lib<P: AsRef<Path>>(&self, pf: &GLPixelFormat, cs: &GLContextSettings, path: P) -> Result<GLContext> {
        self.0.create_gl_context_from_lib(&pf.0, cs, path.as_ref()).map(GLContext)
    }

    // Reply TRUE to WM_QUERYENDSESSION, in which case we then get WM_ENDSESSION.
    // ShutdownBlockReasonDestroy
    pub fn allow_session_termination(&mut self) -> Result<()> {
        self.0.allow_session_termination()
    }
    // Reply FALSE to WM_QUERYENDSESSION.
    // ShutdownBlockReasonCreate
    pub fn disallow_session_termination(&mut self, reason: Option<String>) -> Result<()> {
        self.0.disallow_session_termination(reason)
    }

    pub fn query_best_cursor_size(&self, size_hint: Extent2<u32>) -> Extent2<u32> {
        self.0.query_best_cursor_size(size_hint)
    }
    pub fn create_rgba32_cursor(&self, frame: CursorData) -> Result<Cursor> {
        self.0.create_rgba32_cursor(frame).map(Cursor)
    }
    pub fn create_animated_rgba32_cursor(&self, anim: &[CursorFrame]) -> Result<Cursor> {
        self.0.create_animated_rgba32_cursor(anim).map(Cursor)
    }
    pub fn create_system_cursor(&self, s: SystemCursor) -> Result<Cursor> {
        self.0.create_system_cursor(s).map(Cursor)
    }
}

