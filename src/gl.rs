//! OpenGL-related structures and abstractions.

use std::os::raw::{c_void, c_char};
use os::{OsGLPixelFormat, OsGLContext};
use window::Window;
use error::Result;


/// Hints for Multisample anti-aliasing (MSAA).
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
pub struct GLMsaa {
    /// Number of MSAA buffers. If it's zero, MSAA is disabled.
    pub buffer_count: u32,
    /// Number of samples per pixel. Should be a power of two.
    pub sample_count: u32,
}


/// Settings requested for an OpenGL pixel format.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct GLPixelFormatSettings {
    /// MultiSample AntiAliasing setting.
    pub msaa: GLMsaa,
    /// Number of bits used for storing per-fragment depth values.  
    /// Often set to 24.
    pub depth_bits: u8,
    /// Number of bits used for storing per-fragment "stencil" values.
    /// Often set to `(32 - depth_bits)`.
    pub stencil_bits: u8,
    /// Use double-buffering ? Defaults to `true` because 
    /// not enabling this has been deprecated long ago.
    pub double_buffer: bool,
    /// Requirements "left" and "right" frame buffers instead of a single
    /// frame buffer (which is the default).
    /// Each said "frame buffer" can itself be double-buffered if 
    /// `double_buffer` was set to `true`.
    pub stereo: bool,
    /// Number of bits used for storing the red channel. Often set to 8.
    pub red_bits: u8,
    /// Number of bits used for storing the green channel. Often set to 8.
    pub green_bits: u8,
    /// Number of bits used for storing the blue channel. Often set to 8.
    pub blue_bits: u8,
    /// Number of bits used for storing the alpha channel. Often set to 8.
    pub alpha_bits: u8,
    /// Some platforms support translucent OpenGL windows.
    /// On Windows, this corresponds to the WGL_TRANSPARENT_ARB context attribute.
    /// 
    /// This is `false` by default because alpha-blended windows are relatively niche.
    pub transparent: bool,
    /// Number of bits used for storing the red channel in the accumulation buffer, if any.
    pub accum_red_bits: u8,
    /// Number of bits used for storing the green channel in the accumulation buffer, if any.
    pub accum_green_bits: u8,
    /// Number of bits used for storing the blue channel in the accumulation buffer, if any.
    pub accum_blue_bits: u8,
    /// Number of bits used for storing the alpha channel in the accumulation buffer, if any.
    pub accum_alpha_bits: u8,
    /// Number of auxiliary image buffers.  
    /// This was deprecated since OpenGL 3.0.
    /// 
    /// See [The relevant section on the OpenGL
    /// wiki](https://www.khronos.org/opengl/wiki/Default_Framebuffer#Removed_buffer_images).
    pub aux_buffers: u8,
}

impl Default for GLPixelFormatSettings {
    fn default() -> Self {
        Self {
            msaa: Default::default(),
            depth_bits: 24,
            stencil_bits: 8,
            double_buffer: true,
            stereo: false,
            red_bits: 8,
            green_bits: 8,
            blue_bits: 8,
            alpha_bits: 8,
            transparent: false,
            accum_red_bits: 0,
            accum_green_bits: 0,
            accum_blue_bits: 0,
            accum_alpha_bits: 0,
            aux_buffers: 0,
        }
    }
}

/// OS-specific OpenGL pixel format.
#[derive(Debug)]
pub struct GLPixelFormat(pub(crate) OsGLPixelFormat);

pub trait GLPixelFormatChooser {
    fn settings(&self) -> &GLPixelFormatSettings;
    fn choose(&self, pf: &[GLPixelFormat]) -> usize { 0 }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct GLDefaultPixelFormatChooser<'a>(&'a GLPixelFormatSettings);

impl<'a> From<&'a GLPixelFormatSettings> for GLDefaultPixelFormatChooser<'a> {
    fn from(s: &'a GLPixelFormatSettings) -> Self {
        GLDefaultPixelFormatChooser(s)
    }
}

impl<'a> GLPixelFormatChooser for GLDefaultPixelFormatChooser<'a> {
    fn settings(&self) -> &GLPixelFormatSettings { self.0 }
}

/// Either Desktop GL or OpenGL ES.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum GLVariant {
    Desktop,
    ES,
}

impl GLVariant {
    #[allow(missing_docs)]
    pub fn is_desktop(&self) -> bool { self == &GLVariant::Desktop }
    #[allow(missing_docs)]
    pub fn is_es(&self) -> bool { self == &GLVariant::ES }
}

/// Convenience struct for representing an OpenGL version.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct GLVersion {
    /// Variant; Desktop or ES.
    pub variant: GLVariant,
    /// Major version number.
    pub major: u8,
    /// Minor version number.
    pub minor: u8,
}

impl GLVersion {
    /// Create a desktop GL version from major and minor version numbers.
    pub fn new_desktop(major: u8, minor: u8) -> Self { Self { variant: GLVariant::Desktop, major, minor, } }
    /// Create a GL ES version from major and minor version numbers.
    pub fn new_es(major: u8, minor: u8) -> Self { Self { variant: GLVariant::ES, major, minor, } }
    #[allow(missing_docs)]
    pub fn is_desktop(&self) -> bool { self.variant.is_desktop() }
    #[allow(missing_docs)]
    pub fn is_es(&self) -> bool { self.variant.is_es() }
}

/// Since OpenGL 3.2, the profile for an OpenGL context is either "core" 
/// or "compatibility".  
/// 
/// See [the relevant entry of the OpenGL wiki](https://www.khronos.org/opengl/wiki/Core_And_Compatibility_in_Contexts)
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum GLProfile {
    Core,
    Compatibility,
}

impl Default for GLProfile {
    fn default() -> Self {
        GLProfile::Compatibility
    }
}

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum GLContextResetNotificationStrategy {
    NoResetNotification,
    LoseContextOnReset,
}


/// Settings requested for an OpenGL context.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct GLContextSettings {
    /// Hints the OpenGL version to use.
    pub version: GLVersion,
    /// Only used when the requested OpenGL version is 3.2 or
    /// greater.
    pub profile: GLProfile,
    /// Do we want a debug context ?
    pub debug: bool,
    /// Only used when the requested OpenGL version is 3.0 or 
    /// greater.
    pub forward_compatible: bool, // 3.0+
    /// Enables the "robust access" bit in context flags, if the backend
    /// supports the extension.
    pub robust_access: Option<GLContextResetNotificationStrategy>,
}

/// Wrapper around a platform-specific OpenGL Context.
#[derive(Debug)]
pub struct GLContext(pub(crate) OsGLContext);


/// The interval at which OpenGL buffers are swapped.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GLSwapInterval {
    /// Vertical sync : frames are synchronized with the monitor's refresh 
    /// rate. This is the default.
    VSync,
    /// Immediate frame updates. May make your GPU melt if you don't limit
    /// the FPS.
    Immediate,
    /// Quoting SDL2's docs:  
    /// Late swap tearing works the same as vsync, but if you've 
    /// already missed the vertical
    /// retrace for a given frame, it swaps buffers immediately, which
    /// might be less jarring for
    /// the user during occasional framerate drops.  
    ///   
    /// Late swap tearing is implemented for some glX drivers with
    /// GLX_EXT_swap_control_tear and for some Windows drivers with
    /// WGL_EXT_swap_control_tear.
    LateSwapTearing,
    /// Passed directly as the value for the backend's GL `SwapInterval()`
    /// function. Specifies the number of VBlanks to wait for before presenting. 
    ///
    /// It can be negative - if so, it's a late swap tearing hint and
    /// its absolute value is considered.  
    /// See the `LateSwapTearing` variant of this enum, and for instance the
    /// `GLX_EXT_swap_control_tear` spec.
    /// 
    /// Example meanings of the value:
    ///
    /// - `2`: Vsync/2 (e.g at 60 Hz, will swap buffers 30 times per second.);
    /// - `1`: Vsync (e.g at 60 Hz, will swap buffers 60 times per second.);
    /// - `0`: Immediate updates;
    /// - `-1`: VSync with late swap tearing;
    /// - `-2`: VSync/2 with late swap tearing;
    /// - etc...  
    Interval(i32),
}

impl Default for GLSwapInterval {
    fn default() -> Self {
        GLSwapInterval::VSync
    }
}

impl GLContext {
    /// Retrieves the OpenGL function pointer for the given name.
    pub unsafe fn get_proc_address(&self, name: *const c_char) -> *const c_void {
        self.0.get_proc_address(name)
    }
    /// Retrieves the OpenGL function pointer for the given name.
    pub fn proc_address(&self, name_str: &str) -> *const c_void {
        let name = name_str.as_bytes();
        let mut cstr = [0_u8; 256];
        assert!(name.len() < cstr.len());
        unsafe {
            ::std::ptr::copy_nonoverlapping(name.as_ptr(), cstr.as_mut_ptr(), name.len());
            self.get_proc_address(cstr.as_ptr() as _)
        }
    }
}

impl Window {
    /// Creates an OpenGL context using the given context settings and pixel format.
    pub fn create_gl_context(&self, settings: &GLContextSettings) -> Result<GLContext> {
        self.0.create_gl_context(settings).map(GLContext)
    }
    /// Makes this `GLContext` current for this thread and window.  
    /// The window **must** have been created with the exact same `GLPixelFormat` from which
    /// this `GLContext` was created.
    ///
    /// This is not a method of `Window`, because a `GLContext` can be only current
    /// to at most one render target at a time.
    pub fn make_gl_context_current(&self, c: Option<&GLContext>) -> Result<()> {
        self.0.make_gl_context_current(c.map(|c| &c.0))
    }
    /// Presents an OpenGL frame (commonly referred to as "swapping buffers").
    ///
    /// There must be a current `GLContext` which targets this window.
    pub fn gl_swap_buffers(&self) -> Result<()> {
        self.0.gl_swap_buffers()
    }
    /// Sets the OpenGL swap interval for this window.
    /// 
    /// There must be a current `GLContext` which targets this window.
    pub fn gl_set_swap_interval(&self, interval: GLSwapInterval) -> Result<()> {
        self.0.gl_set_swap_interval(interval)
    }
}

