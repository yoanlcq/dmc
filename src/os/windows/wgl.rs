use std::os::raw::*;
use std::mem;
use std::ptr;
use std::ffi::CStr;
use std::fmt::{self, Debug, Formatter};
use super::{OsSharedContext, winapi_utils::*};
use error::Result;

impl OsSharedContext {
    pub fn wgl(&self) -> Result<&Wgl> {
        self.wgl.as_ref().map_err(Clone::clone)
    }
}

// extern "C" fns implement Debug, but not extern "system" fns. Urgh.
#[allow(non_snake_case)]
#[derive(Copy, Clone, Default)]
pub struct WglFns {
    pub wglGetExtensionsStringARB: Option<unsafe extern "system" fn(HDC) -> *const c_char>,
    pub wglCreateContextAttribsARB: Option<unsafe extern "system" fn(HDC, HGLRC, *const c_int) -> HGLRC>,
    pub wglChoosePixelFormatARB: Option<unsafe extern "system" fn(HDC, *const c_int, *const f32, UINT, *mut c_int, *mut UINT) -> BOOL>,
    pub wglSwapIntervalEXT: Option<unsafe extern "system" fn(c_int) -> BOOL>,
    pub wglGetSwapIntervalEXT: Option<unsafe extern "system" fn() -> c_int>,
}

impl Debug for WglFns {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        #[allow(non_snake_case)]
        let &Self {
            wglGetExtensionsStringARB,
            wglCreateContextAttribsARB,
            wglChoosePixelFormatARB,
            wglSwapIntervalEXT,
            wglGetSwapIntervalEXT,
        } = self;
        f.debug_struct("WglFns")
            .field("wglGetExtensionsStringARB",  &wglGetExtensionsStringARB .map(|f| f as *const c_void))
            .field("wglCreateContextAttribsARB", &wglCreateContextAttribsARB.map(|f| f as *const c_void))
            .field("wglChoosePixelFormatARB",    &wglChoosePixelFormatARB   .map(|f| f as *const c_void))
            .field("wglSwapIntervalEXT",         &wglSwapIntervalEXT        .map(|f| f as *const c_void))
            .field("wglGetSwapIntervalEXT",      &wglGetSwapIntervalEXT     .map(|f| f as *const c_void))
            .finish()
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Copy, Clone, Default)]
pub struct Wgl {
    pub fns: WglFns,
    pub WGL_ARB_create_context: bool,
    pub WGL_ARB_create_context_profile: bool,
    pub WGL_ARB_create_context_robustness: bool,
    pub WGL_EXT_create_context_es_profile: bool,
    pub WGL_EXT_create_context_es2_profile: bool,
    pub WGL_ARB_multisample: bool,
    pub WGL_EXT_multisample: bool,
    pub WGL_EXT_pixel_format: bool,
    pub WGL_ARB_pixel_format: bool,
    pub WGL_ARB_pixel_format_float: bool,
    pub WGL_ARB_robustness_application_isolation: bool,
    pub WGL_ARB_robustness_share_group_isolation: bool,
    pub WGL_EXT_swap_control: bool,
    pub WGL_EXT_swap_control_tear: bool,
    pub WGL_EXT_colorspace: bool,
    pub WGL_EXT_framebuffer_sRGB: bool,
    pub WGL_ARB_framebuffer_sRGB: bool,
}

impl Wgl {
    pub fn new() -> Result<Self> {
        // The plan: Create a legacy OpenGL context (needs a temporary window, etc), get the WGL function pointers, then get rid of everything.
        unsafe {
            let hinstance = GetModuleHandleW(ptr::null());

            let classname = to_wide_with_nul("DMC dummy OpenGL context window");
            assert!(classname.len() < 256);
            
            let wclass = WNDCLASSEXW {
                cbSize: mem::size_of::<WNDCLASSEXW>() as _,
                hInstance: hinstance,
                lpfnWndProc: Some(DefWindowProcW),
                lpszClassName: classname.as_ptr(),
                style: CS_OWNDC,
                cbClsExtra: 0,
                cbWndExtra: 0,
                hIcon: ptr::null_mut(),
                hIconSm: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
            };
            let class_atom = RegisterClassExW(&wclass);
            if class_atom == 0 {
                return winapi_fail("RegisterClassExW");
            }

            let tmp_hwnd = CreateWindowExW(
                WS_EX_OVERLAPPEDWINDOW,
                MAKEINTATOM(class_atom),
                ptr::null_mut(), // No title
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT, // x
                CW_USEDEFAULT, // y
                CW_USEDEFAULT, // w
                CW_USEDEFAULT, // h
                ptr::null_mut(), // No parent
                ptr::null_mut(), // No menu
                hinstance,
                ptr::null_mut(), // No custom data pointer
            );
            assert!(!tmp_hwnd.is_null());

            let tmp_window_hdc = GetDC(tmp_hwnd);
            assert!(!tmp_window_hdc.is_null());

            let pfd = PIXELFORMATDESCRIPTOR {
                nSize: mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
                nVersion: 1,
                dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER, 
                iPixelType: PFD_TYPE_RGBA,
                cColorBits: 32,
                cRedBits: 0,
                cRedShift: 0,
                cGreenBits: 0,
                cGreenShift: 0,
                cBlueBits: 0,
                cBlueShift: 0,
                cAlphaBits: 0,
                cAlphaShift: 0,
                cAccumBits: 0,
                cAccumRedBits: 0,
                cAccumGreenBits: 0,
                cAccumBlueBits: 0,
                cAccumAlphaBits: 0,
                cDepthBits: 24,
                cStencilBits: 8,
                cAuxBuffers: 0,
                iLayerType: PFD_MAIN_PLANE,
                bReserved: 0,
                dwLayerMask: 0,
                dwVisibleMask: 0,
                dwDamageMask: 0,
            };
            let i_pixel_format = ChoosePixelFormat(tmp_window_hdc, &pfd);
            assert_ne!(i_pixel_format, 0);
            let is_ok = SetPixelFormat(tmp_window_hdc, i_pixel_format, &pfd);
            assert_ne!(is_ok, FALSE);
            let tmp_hglrc = wglCreateContext(tmp_window_hdc);
            assert!(!tmp_hglrc.is_null());
            let is_ok = wglMakeCurrent(tmp_window_hdc, tmp_hglrc);
            assert_ne!(is_ok, FALSE);

            unsafe fn get_fn(name: &[u8]) -> Option<&c_void> {
                assert_eq!(&0, name.last().unwrap());
                match wglGetProcAddress(name.as_ptr() as _) as usize {
                    0 => None,
                    f => Some(mem::transmute(f)),
                }
            };

            let mut wgl = Wgl {
                fns: WglFns {
                    wglGetExtensionsStringARB: mem::transmute(get_fn(b"wglGetExtensionsStringARB\0")),
                    wglCreateContextAttribsARB: mem::transmute(get_fn(b"wglCreateContextAttribsARB\0")),
                    wglChoosePixelFormatARB: mem::transmute(get_fn(b"wglChoosePixelFormatARB\0")),
                    wglSwapIntervalEXT: mem::transmute(get_fn(b"wglSwapIntervalEXT\0")),
                    wglGetSwapIntervalEXT: mem::transmute(get_fn(b"wglGetSwapIntervalEXT\0")),
                },
                .. Default::default()
            };

            if let Some(f) = wgl.fns.wglGetExtensionsStringARB {
                let exts = f(tmp_window_hdc);
                assert!(!exts.is_null());
                let exts = CStr::from_ptr(exts).to_string_lossy();
                for ext in exts.split(' ') {
                    match ext {
                        "WGL_ARB_create_context" => wgl.WGL_ARB_create_context = true,
                        "WGL_ARB_create_context_profile" => wgl.WGL_ARB_create_context_profile = true,
                        "WGL_ARB_create_context_robustness" => wgl.WGL_ARB_create_context_robustness = true,
                        "WGL_EXT_create_context_es_profile" => wgl.WGL_EXT_create_context_es_profile = true,
                        "WGL_EXT_create_context_es2_profile" => wgl.WGL_EXT_create_context_es2_profile = true,
                        "WGL_ARB_multisample" => wgl.WGL_ARB_multisample = true,
                        "WGL_EXT_multisample" => wgl.WGL_EXT_multisample = true,
                        "WGL_EXT_pixel_format" => wgl.WGL_EXT_pixel_format = true,
                        "WGL_ARB_pixel_format" => wgl.WGL_ARB_pixel_format = true,
                        "WGL_ARB_pixel_format_float" => wgl.WGL_ARB_pixel_format_float = true,
                        "WGL_ARB_robustness_application_isolation" => wgl.WGL_ARB_robustness_application_isolation = true,
                        "WGL_ARB_robustness_share_group_isolation" => wgl.WGL_ARB_robustness_share_group_isolation = true,
                        "WGL_EXT_swap_control" => wgl.WGL_EXT_swap_control = true,
                        "WGL_EXT_swap_control_tear" => wgl.WGL_EXT_swap_control_tear = true,
                        "WGL_EXT_colorspace" => wgl.WGL_EXT_colorspace = true,
                        "WGL_EXT_framebuffer_sRGB" => wgl.WGL_EXT_framebuffer_sRGB = true,
                        "WGL_ARB_framebuffer_sRGB" => wgl.WGL_ARB_framebuffer_sRGB = true,
                        _ => (),
                    }
                }
            }

            if wgl.WGL_ARB_create_context {
                assert!(wgl.fns.wglCreateContextAttribsARB.is_some());
            }
            if wgl.WGL_ARB_pixel_format {
                assert!(wgl.fns.wglChoosePixelFormatARB.is_some());
            }
            if wgl.WGL_EXT_swap_control {
                assert!(wgl.fns.wglSwapIntervalEXT.is_some());
                assert!(wgl.fns.wglGetSwapIntervalEXT.is_some());
            }

            // Now we've got the function pointers, get rid of the tmp window and hdc
            let is_ok = wglMakeCurrent(tmp_window_hdc, ptr::null_mut());
            assert_ne!(is_ok, FALSE);
            let is_ok = wglDeleteContext(tmp_hglrc);
            assert_ne!(is_ok, FALSE);
            // NOTE: Don't Release or Delete the HDC. Not needed and will fail because of CS_OWNDC.
            let is_ok = DestroyWindow(tmp_hwnd);
            assert_ne!(is_ok, FALSE);
            let is_ok = UnregisterClassW(MAKEINTATOM(class_atom), hinstance);
            assert_ne!(is_ok, FALSE);

            Ok(wgl)
        }
    }
}

#[allow(dead_code)]
#[allow(non_upper_case_globals)]
pub mod consts {
    use super::*;

    // NOTE: Not everything's in it. Additions welcome!

    pub const WGL_CONTEXT_DEBUG_BIT_ARB: c_int =         0x00000001;
    pub const WGL_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB: c_int = 0x00000002;
    pub const WGL_CONTEXT_MAJOR_VERSION_ARB: c_int =     0x2091;
    pub const WGL_CONTEXT_MINOR_VERSION_ARB: c_int =     0x2092;
    pub const WGL_CONTEXT_LAYER_PLANE_ARB: c_int =       0x2093;
    pub const WGL_CONTEXT_FLAGS_ARB: c_int =             0x2094;
    pub const ERROR_INVALID_VERSION_ARB: c_int =         0x2095;
    pub const WGL_CONTEXT_OPENGL_NO_ERROR_ARB: c_int =   0x31B3;
    pub const WGL_CONTEXT_PROFILE_MASK_ARB: c_int =      0x9126;
    pub const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: c_int =  0x00000001;
    pub const WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB: c_int = 0x00000002;
    pub const ERROR_INVALID_PROFILE_ARB: c_int =         0x2096;
    pub const WGL_CONTEXT_ROBUST_ACCESS_BIT_ARB: c_int = 0x00000004;
    pub const WGL_LOSE_CONTEXT_ON_RESET_ARB: c_int =     0x8252;
    pub const WGL_CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB: c_int = 0x8256;
    pub const WGL_NO_RESET_NOTIFICATION_ARB: c_int =     0x8261;
    pub const WGL_FRAMEBUFFER_SRGB_CAPABLE_ARB: c_int =  0x20A9;

    pub const WGL_SAMPLE_BUFFERS_ARB: c_int =            0x2041;
    pub const WGL_SAMPLES_ARB: c_int =                   0x2042;

    pub const WGL_NUMBER_PIXEL_FORMATS_ARB: c_int =      0x2000;
    pub const WGL_DRAW_TO_WINDOW_ARB: c_int =            0x2001;
    pub const WGL_DRAW_TO_BITMAP_ARB: c_int =            0x2002;
    pub const WGL_ACCELERATION_ARB: c_int =              0x2003;
    pub const WGL_NEED_PALETTE_ARB: c_int =              0x2004;
    pub const WGL_NEED_SYSTEM_PALETTE_ARB: c_int =       0x2005;
    pub const WGL_SWAP_LAYER_BUFFERS_ARB: c_int =        0x2006;
    pub const WGL_SWAP_METHOD_ARB: c_int =               0x2007;
    pub const WGL_NUMBER_OVERLAYS_ARB: c_int =           0x2008;
    pub const WGL_NUMBER_UNDERLAYS_ARB: c_int =          0x2009;
    pub const WGL_TRANSPARENT_ARB: c_int =               0x200A;
    pub const WGL_TRANSPARENT_RED_VALUE_ARB: c_int =     0x2037;
    pub const WGL_TRANSPARENT_GREEN_VALUE_ARB: c_int =   0x2038;
    pub const WGL_TRANSPARENT_BLUE_VALUE_ARB: c_int =    0x2039;
    pub const WGL_TRANSPARENT_ALPHA_VALUE_ARB: c_int =   0x203A;
    pub const WGL_TRANSPARENT_INDEX_VALUE_ARB: c_int =   0x203B;
    pub const WGL_SHARE_DEPTH_ARB: c_int =               0x200C;
    pub const WGL_SHARE_STENCIL_ARB: c_int =             0x200D;
    pub const WGL_SHARE_ACCUM_ARB: c_int =               0x200E;
    pub const WGL_SUPPORT_GDI_ARB: c_int =               0x200F;
    pub const WGL_SUPPORT_OPENGL_ARB: c_int =            0x2010;
    pub const WGL_DOUBLE_BUFFER_ARB: c_int =             0x2011;
    pub const WGL_STEREO_ARB: c_int =                    0x2012;
    pub const WGL_PIXEL_TYPE_ARB: c_int =                0x2013;
    pub const WGL_COLOR_BITS_ARB: c_int =                0x2014;
    pub const WGL_RED_BITS_ARB: c_int =                  0x2015;
    pub const WGL_RED_SHIFT_ARB: c_int =                 0x2016;
    pub const WGL_GREEN_BITS_ARB: c_int =                0x2017;
    pub const WGL_GREEN_SHIFT_ARB: c_int =               0x2018;
    pub const WGL_BLUE_BITS_ARB: c_int =                 0x2019;
    pub const WGL_BLUE_SHIFT_ARB: c_int =                0x201A;
    pub const WGL_ALPHA_BITS_ARB: c_int =                0x201B;
    pub const WGL_ALPHA_SHIFT_ARB: c_int =               0x201C;
    pub const WGL_ACCUM_BITS_ARB: c_int =                0x201D;
    pub const WGL_ACCUM_RED_BITS_ARB: c_int =            0x201E;
    pub const WGL_ACCUM_GREEN_BITS_ARB: c_int =          0x201F;
    pub const WGL_ACCUM_BLUE_BITS_ARB: c_int =           0x2020;
    pub const WGL_ACCUM_ALPHA_BITS_ARB: c_int =          0x2021;
    pub const WGL_DEPTH_BITS_ARB: c_int =                0x2022;
    pub const WGL_STENCIL_BITS_ARB: c_int =              0x2023;
    pub const WGL_AUX_BUFFERS_ARB: c_int =               0x2024;
    pub const WGL_NO_ACCELERATION_ARB: c_int =           0x2025;
    pub const WGL_GENERIC_ACCELERATION_ARB: c_int =      0x2026;
    pub const WGL_FULL_ACCELERATION_ARB: c_int =         0x2027;
    pub const WGL_SWAP_EXCHANGE_ARB: c_int =             0x2028;
    pub const WGL_SWAP_COPY_ARB: c_int =                 0x2029;
    pub const WGL_SWAP_UNDEFINED_ARB: c_int =            0x202A;
    pub const WGL_TYPE_RGBA_ARB: c_int =                 0x202B;
    pub const WGL_TYPE_COLORINDEX_ARB: c_int =           0x202C;

    pub const WGL_TYPE_RGBA_FLOAT_ARB: c_int =           0x21A0;

    pub const WGL_FRAMEBUFFER_SRGB_CAPABLE_EXT: c_int =   0x20A9;

    pub const WGL_DEPTH_FLOAT_EXT: c_int =                0x2040;

    pub const WGL_CONTEXT_ES_PROFILE_BIT_EXT: c_int =     0x00000004;
    pub const WGL_CONTEXT_ES2_PROFILE_BIT_EXT: c_int =    0x00000004;

    pub const WGL_COLORSPACE_EXT: c_int =                 0x3087;
    pub const WGL_COLORSPACE_SRGB_EXT: c_int =            0x3089;
    pub const WGL_COLORSPACE_LINEAR_EXT: c_int =          0x308A;
}