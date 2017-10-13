extern crate env_logger;
extern crate dmc;
extern crate gl;
#[macro_use]
extern crate log;

use std::time::Duration;
use std::thread::sleep;
use std::ptr;
use std::ffi::*;

use dmc::display::*;
use dmc::display::window::Settings as WindowSettings;
use dmc::option_alternatives::*;
use dmc::Extent2;

use gl::types::*;

fn main() {
    std::env::set_var("RUST_LOG", "info");
    std::env::set_var("RUST_BACKTRACE", "full");

    let gl_pf_settings = GLPixelFormatSettings {
        msaa: GLMsaa {
            buffer_count: 1,
            sample_count: 4,
        },
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
    };
    let gl_ctx_settings = GLContextSettings {
        version: Decision::Manual(GLVersion::GL_3_2),
        debug: true,
        forward_compatible: true, // 3.0+
        profile: Decision::Manual(GLProfile::Compatibility),
        robust_access: None,
    };

    env_logger::init().unwrap();
    let display = Display::open().expect("Could not open display!");
    let gl_pixel_format = display.choose_gl_pixel_format(&gl_pf_settings).expect("Couldn't choose pixel format!");
    let window_settings = WindowSettings {
        mode: window::Mode::FixedSize(Extent2 { w: 400, h: 300 }),
        opengl: Some(&gl_pixel_format),
        resizable: true,
        allow_high_dpi: true,
        fully_opaque: true,
    };
    let window = display.create_window(&window_settings).expect("Couldn't create window!");
    let gl_ctx = display.create_gl_context(&gl_pixel_format, &gl_ctx_settings).expect("Couldn't create GL context!");
    let swap_chain = gl_ctx.make_current(&window);

    gl::load_with(|s| match gl_ctx.get_proc_address(s) {
        Some(p) => {
            info!("Loaded `{}`", s);
            p as *const _
        },
        None => {
            info!("Couldn't load `{}`", s);
            ptr::null()
        },
    });

    // TODO: Let's log as much info as we can from the GL context!

    unsafe {
        let mut ctxflags: GLint = 0;
        let mut ctxpmask: GLint = 0;
        let mut depth_bits: GLint = 0;
        let mut stencil_bits: GLint = 0;
        let mut double_buffer: GLboolean = 0;
        let mut stereo_buffers: GLboolean = 0;
        gl::GetIntegerv(gl::CONTEXT_FLAGS, &mut ctxflags);
        gl::GetIntegerv(gl::CONTEXT_PROFILE_MASK, &mut ctxpmask);
        gl::GetFramebufferAttachmentParameteriv(gl::FRAMEBUFFER, gl::DEPTH, 
                gl::FRAMEBUFFER_ATTACHMENT_DEPTH_SIZE, &mut depth_bits);
        gl::GetFramebufferAttachmentParameteriv(gl::FRAMEBUFFER, gl::STENCIL, 
                gl::FRAMEBUFFER_ATTACHMENT_STENCIL_SIZE, &mut stencil_bits);
        gl::GetBooleanv(gl::DOUBLEBUFFER, &mut double_buffer);
        gl::GetBooleanv(gl::STEREO, &mut stereo_buffers);

        let ctxflags = ctxflags as GLuint;
        let ctxpmask = ctxpmask as GLuint;

        let gl_version    = CStr::from_ptr(gl::GetString(gl::VERSION) as _).to_string_lossy();
        let gl_renderer   = CStr::from_ptr(gl::GetString(gl::RENDERER) as _).to_string_lossy();
        let gl_vendor     = CStr::from_ptr(gl::GetString(gl::VENDOR) as _).to_string_lossy();
        let glsl_version  = CStr::from_ptr(gl::GetString(gl::SHADING_LANGUAGE_VERSION) as _).to_string_lossy();
        let gl_extensions = CStr::from_ptr(gl::GetString(gl::EXTENSIONS) as _).to_string_lossy();


        // TODO: report to gl crate.
        let CONTEXT_FLAG_NO_ERROR_BIT_KHR: GLuint = 0x00000008;

        info!(
"--- Active OpenGL context settings ---
    Version             : {}
    Renderer            : {}
    Vendor              : {}
    GLSL version        : {}
    Profile flags       : {} (bits: 0b{:08b})
    Context flags       : {}{}{}{} (bits: {:08b})
    Double buffering    : {}
    Stereo buffers      : {}
    Depth buffer bits   : {}
    Stencil buffer bits : {}
    Extesions           : {}",
            gl_version, gl_renderer, gl_vendor, glsl_version,
            if ctxpmask & gl::CONTEXT_CORE_PROFILE_BIT != 0 {
                "core"
            } else if ctxpmask & gl::CONTEXT_COMPATIBILITY_PROFILE_BIT != 0 {
                "compatibility"
            } else { "" },
            ctxpmask,
if ctxflags & gl::CONTEXT_FLAG_FORWARD_COMPATIBLE_BIT != 0 { "forward_compatible " } else {""},
if ctxflags & gl::CONTEXT_FLAG_DEBUG_BIT != 0 { "debug " } else {""},
if ctxflags & gl::CONTEXT_FLAG_ROBUST_ACCESS_BIT != 0 { "robust_access " } else {""},
if ctxflags &     CONTEXT_FLAG_NO_ERROR_BIT_KHR != 0 { "no_error " } else {""},
            ctxflags,
            double_buffer, stereo_buffers, depth_bits, stencil_bits,
            gl_extensions
        );
    }
    /*
    {
        GLint num_glfmts;
        glGetIntegerv(GL_NUM_COMPRESSED_TEXTURE_FORMATS, &num_glfmts);
        GLint *fmts = fe_mem_heapalloc(num_glfmts, GLint, "");
        fe_dbg_hope(fmts);
        glGetIntegerv(GL_COMPRESSED_TEXTURE_FORMATS, fmts);
        fe_logi(TAG, "\n    Compressed texture formats :\n\n");
        for(i=0 ; i<num_glfmts ; i++)
            fe_logi(TAG, "0x%.4"PRIx32": %s\n", (int32_t)fmts[i],
                    fe_gl_tc_format_to_name(fmts[i]));
        fe_mem_heapfree(fmts);
    }

    {
        fe_logi(TAG, "\n    Limits :\n\n");
        GLint val;
#define HELPER(CST, req) \
        glGetIntegerv(GL_MAX_##CST, &val); \
        fe_logi(TAG, "GL_MAX_%-28s : %d (standard: %d)\n", #CST, (int)val, req)
        HELPER(RENDERBUFFER_SIZE           ,   1);
        HELPER(TEXTURE_IMAGE_UNITS         ,   8);
        HELPER(COMBINED_TEXTURE_IMAGE_UNITS,   8);
        HELPER(TEXTURE_SIZE                ,  64);
        HELPER(CUBE_MAP_TEXTURE_SIZE       ,  16);
        HELPER(VERTEX_ATTRIBS              ,   8);
        HELPER(VERTEX_TEXTURE_IMAGE_UNITS  ,   0);
        HELPER(VERTEX_UNIFORM_VECTORS      , 128);
        HELPER(VARYING_VECTORS             ,   8);
        HELPER(FRAGMENT_UNIFORM_VECTORS    ,  16);
#undef HELPER
        GLint dims[2];
        glGetIntegerv(GL_MAX_VIEWPORT_DIMS, dims);
        fe_logi(TAG, "GL_MAX_%-28s : %dx%d\n", "VIEWPORT_DIMS", 
                (int)dims[0], (int)dims[1]);
    }
    */

    if swap_chain.set_swap_interval(GLSwapInterval::LateSwapTearing).is_err() {
        if swap_chain.set_swap_interval(GLSwapInterval::VSync).is_err() {
            swap_chain.set_swap_interval(GLSwapInterval::LimitFps(60)).unwrap();
            info!("Set swap interval to Manual: 60 FPS.");
        } else {
            info!("Set swap interval to VSync.");
        }
    } else {
        info!("Set swap interval to Late Swap Tearing");
    }

    window.set_title("Three");
    unsafe {
        gl::ClearColor(1f32, 0f32, 0f32, 1f32);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }
    // NOTE: show() before present(), because otherwise presenting won't take place the first time.
    window.show();
    swap_chain.present();
    sleep(Duration::from_secs(1));

    window.set_title("Two");
    unsafe {
        gl::ClearColor(0f32, 1f32, 0f32, 1f32);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }
    swap_chain.present();
    sleep(Duration::from_secs(1));

    window.set_title("One");
    unsafe {
        gl::ClearColor(0f32, 0f32, 1f32, 1f32);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }
    swap_chain.present();
    sleep(Duration::from_secs(1));
}
