extern crate log;
extern crate env_logger;
extern crate dmc;

mod early;

use dmc::{Context, WindowSettings, WindowMode};

fn main() {
    early::early();
    let context = Context::new().expect("Could not create context!");
    let _window = context.create_window(&WindowSettings {
        mode: WindowMode::FixedSize(400, 300),
        opengl: None,
        resizable: true,
        allow_high_dpi: true,
        fully_opaque: true,
    });
}
