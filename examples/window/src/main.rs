extern crate env_logger;
extern crate dmc;
extern crate gl;

use std::time::Duration;
use std::thread::sleep;
use std::ptr;

use dmc::display::*;
use dmc::display::window::Settings as WindowSettings;
use dmc::option_alternatives::*;
use dmc::Extent2;

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
        Some(p) => p as *const _,
        None => ptr::null(),
    });

    // TODO: Let's log as much info as we can from the GL context!

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
