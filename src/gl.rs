use Semver;
use Decision;
use os::*;
use window::*;
use std::os::raw::c_char;


#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
pub struct GLMsaa {
    /// Number of MSAA buffers. If it's zero, no MSAA takes place.
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
    /// Often set to 32-`depth_bits`.
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


/// Either Desktop GL or OpenGL ES.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum GLVariant {
    Desktop,
    ES,
}

/// Known OpenGL version numbers.
/// 
/// If you're looking for WebGL, know that WebGL 
/// 1.0 maps closely to ES 2.0, and WebGL 2.0 maps closely to ES 3.0.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[allow(non_camel_case_types, missing_docs)]
#[repr(u16)]
pub enum GLVersion {
    GL(Semver),
    ES(Semver),

    GL_4_5,
    GL_4_4,
    GL_4_3,
    GL_4_2,
    GL_4_1,
    GL_4_0,
    GL_3_3,
    GL_3_2,
    GL_3_1,
    GL_3_0,
    GL_2_1,
    GL_2_0,
    GL_1_5,
    GL_1_4,
    GL_1_3,
    GL_1_2_1,
    GL_1_2,
    GL_1_1,

    ES_3_2,
    ES_3_1,
    ES_3_0,
    ES_2_0,
    ES_1_1,
    ES_1_0,
}

impl GLVersion {
    #[allow(missing_docs)]
    // If None is returned, the user can still build a manual version
    // with the GL() and ES() variants.
    pub fn try_from_semver(v: &(GLVariant, Semver)) -> Option<Self> {
        let &(variant, Semver {major, minor, patch}) = v;
        match (variant, major, minor, patch) {
           (GLVariant::Desktop, 4,5,0) => Some(GLVersion::GL_4_5  ),
           (GLVariant::Desktop, 4,4,0) => Some(GLVersion::GL_4_4  ),
           (GLVariant::Desktop, 4,3,0) => Some(GLVersion::GL_4_3  ),
           (GLVariant::Desktop, 4,2,0) => Some(GLVersion::GL_4_2  ),
           (GLVariant::Desktop, 4,1,0) => Some(GLVersion::GL_4_1  ),
           (GLVariant::Desktop, 4,0,0) => Some(GLVersion::GL_4_0  ),
           (GLVariant::Desktop, 3,3,0) => Some(GLVersion::GL_3_3  ),
           (GLVariant::Desktop, 3,2,0) => Some(GLVersion::GL_3_2  ),
           (GLVariant::Desktop, 3,1,0) => Some(GLVersion::GL_3_1  ),
           (GLVariant::Desktop, 3,0,0) => Some(GLVersion::GL_3_0  ),
           (GLVariant::Desktop, 2,1,0) => Some(GLVersion::GL_2_1  ),
           (GLVariant::Desktop, 2,0,0) => Some(GLVersion::GL_2_0  ),
           (GLVariant::Desktop, 1,5,0) => Some(GLVersion::GL_1_5  ),
           (GLVariant::Desktop, 1,4,0) => Some(GLVersion::GL_1_4  ),
           (GLVariant::Desktop, 1,3,0) => Some(GLVersion::GL_1_3  ),
           (GLVariant::Desktop, 1,2,1) => Some(GLVersion::GL_1_2_1),
           (GLVariant::Desktop, 1,2,0) => Some(GLVersion::GL_1_2  ),
           (GLVariant::Desktop, 1,1,0) => Some(GLVersion::GL_1_1  ),
           (GLVariant::ES     , 3,2,0) => Some(GLVersion::ES_3_2  ),
           (GLVariant::ES     , 3,1,0) => Some(GLVersion::ES_3_1  ),
           (GLVariant::ES     , 3,0,0) => Some(GLVersion::ES_3_0  ),
           (GLVariant::ES     , 2,0,0) => Some(GLVersion::ES_2_0  ),
           (GLVariant::ES     , 1,1,0) => Some(GLVersion::ES_1_1  ),
           (GLVariant::ES     , 1,0,0) => Some(GLVersion::ES_1_0  ),
           _ => None,
        }
    }
    #[allow(missing_docs)]
    pub fn to_semver(&self) -> (GLVariant, Semver) {
        match *self {
            GLVersion::GL(v)    => (GLVariant::Desktop, v),
            GLVersion::ES(v)    => (GLVariant::ES     , v),
            GLVersion::GL_4_5   => (GLVariant::Desktop, Semver::new(4,5,0)),
            GLVersion::GL_4_4   => (GLVariant::Desktop, Semver::new(4,4,0)),
            GLVersion::GL_4_3   => (GLVariant::Desktop, Semver::new(4,3,0)),
            GLVersion::GL_4_2   => (GLVariant::Desktop, Semver::new(4,2,0)),
            GLVersion::GL_4_1   => (GLVariant::Desktop, Semver::new(4,1,0)),
            GLVersion::GL_4_0   => (GLVariant::Desktop, Semver::new(4,0,0)),
            GLVersion::GL_3_3   => (GLVariant::Desktop, Semver::new(3,3,0)),
            GLVersion::GL_3_2   => (GLVariant::Desktop, Semver::new(3,2,0)),
            GLVersion::GL_3_1   => (GLVariant::Desktop, Semver::new(3,1,0)),
            GLVersion::GL_3_0   => (GLVariant::Desktop, Semver::new(3,0,0)),
            GLVersion::GL_2_1   => (GLVariant::Desktop, Semver::new(2,1,0)),
            GLVersion::GL_2_0   => (GLVariant::Desktop, Semver::new(2,0,0)),
            GLVersion::GL_1_5   => (GLVariant::Desktop, Semver::new(1,5,0)),
            GLVersion::GL_1_4   => (GLVariant::Desktop, Semver::new(1,4,0)),
            GLVersion::GL_1_3   => (GLVariant::Desktop, Semver::new(1,3,0)),
            GLVersion::GL_1_2_1 => (GLVariant::Desktop, Semver::new(1,2,1)),
            GLVersion::GL_1_2   => (GLVariant::Desktop, Semver::new(1,2,0)),
            GLVersion::GL_1_1   => (GLVariant::Desktop, Semver::new(1,1,0)),
            GLVersion::ES_3_2   => (GLVariant::ES     , Semver::new(3,2,0)),
            GLVersion::ES_3_1   => (GLVariant::ES     , Semver::new(3,1,0)),
            GLVersion::ES_3_0   => (GLVariant::ES     , Semver::new(3,0,0)),
            GLVersion::ES_2_0   => (GLVariant::ES     , Semver::new(2,0,0)),
            GLVersion::ES_1_1   => (GLVariant::ES     , Semver::new(1,1,0)),
            GLVersion::ES_1_0   => (GLVariant::ES     , Semver::new(1,0,0)),
        }
    }
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
    /// Hints the OpenGL version to use. Setting it to `Auto` will try to
    /// pick the highest possible or a reasonably modern one.
    pub version: Decision<GLVersion>,
    /// Only used vhen the requested OpenGL version is 3.2 or
    /// greater.
    /// 
    /// If you set it to Auto, the implementation will
    /// attempt to open a Compatibility profile, and if
    /// it fails, open a Core profile.
    pub profile: Decision<GLProfile>,
    /// Do we want a debug context ?
    pub debug: bool,
    /// Only used when the requested OpenGL version is 3.0 or 
    /// greater.
    pub forward_compatible: bool, // 3.0+
    /// Enables the "robust access" bit in context flags, if the backend
    /// supports the extension.
    pub robust_access: Option<GLContextResetNotificationStrategy>,
}

impl Default for GLContextSettings {
    fn default() -> Self {
        Self {
            version: Decision::Auto,
            debug: true,
            forward_compatible: true, // 3.0+
            profile: Default::default(),
            robust_access: None,
        }
    }
}

impl GLContextSettings {
    /// TODO this function checks the correctness of these settings.
    /// For instance, it reports that not using double buffering is 
    /// deprecated.
    pub fn sanitize(self) -> GLContextSettings {
        unimplemented!()
    }
}


/// Wrapper around a platform-specific OpenGL Context.
pub struct GLContext(pub(crate) OsGLContext);

impl GLContext {
    /// Retrieves the OpenGL function pointer for the given name.
    // XXX Will the "C" calling convention be correct in all cases ?
    pub unsafe fn get_proc_address(&self, name: *const c_char) -> Option<OsGLProc> {
        self.0.get_proc_address(name)
    }
}


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
    /// Prevents frames from being presented faster than the given
    /// frames-per-second limit.
    ///
    /// It's rather for convenience since properly setting a swap interval
    /// may not be supported, in which case the FPS skyrockets and the GPU
    /// melts.
    LimitFps(f32),
}

impl Default for GLSwapInterval {
    fn default() -> Self {
        GLSwapInterval::VSync
    }
}

