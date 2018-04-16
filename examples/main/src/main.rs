#![cfg_attr(feature="headless", allow(unreachable_code))]

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate dmc;

mod early;

use std::thread;
use std::time::Duration;
use dmc::{Context, Window, WindowSettings, WindowTypeHint, Extent2, Vec2, Rect};

fn window_test_op<F: FnMut(&Window)>(window: &Window, name: &str, mut f: F, dur: Duration) {
    info!("Testing \"{}\"", name);
    window.set_title(name).unwrap();
    f(window);
    #[cfg(any(target_os="linux", target_os="freebsd", target_os="dragonfly", target_os="openbsd", target_os="netbsd"))]
    early::x11_specific::clear_window_x11(window);
    info!("Position: {}", window.position().unwrap());
    info!("Size: {}", window.size().unwrap());
    match window.canvas_size() {
        Ok(size) => info!("Canvas size: {}", size),
        Err(e) => warn!("Could not get canvas size: {}", e),
    };
    info!("Position and size: {:?}", window.position_and_size().unwrap());
    thread::sleep(dur);
    info!("Done testing \"{}\"", name);
}

fn main() {
    #[cfg(feature="headless")]
    return; // We want this to compile but we won't be able to do anything on headless environments

    early::early();

    let context = Context::new().expect("Could not create context!");
    let window = context.create_window(&WindowSettings {
        position: (0, 0).into(),
        size: (400, 300).into(),
        opengl: None,
        high_dpi: true,
    }).expect("Could not create window!");

    window.set_type_hint(&WindowTypeHint::default()).unwrap();

    window_test_op(&window, "Raise", |w| w.raise().unwrap(), Duration::default());

    window_test_op(&window, "Show", |w| w.show().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Hide", |w| w.hide().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Show", |w| w.show().unwrap(), Duration::from_secs(2));

    window_test_op(&window, "Half opacity", |w| w.set_opacity(0.5).unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Zero opacity", |w| w.set_opacity(0.).unwrap(), Duration::from_secs(2));

    window_test_op(&window, "Set Min Size", |w| {
        w.set_opacity(1.).unwrap();
        w.set_min_size(Extent2::new(500, 500)).unwrap()
    }, Duration::from_secs(2));

    window_test_op(&window, "Set Max Size", |w| {
        w.set_min_size(Extent2::new(0, 0)).unwrap();
        w.set_max_size(Extent2::new(400, 400)).unwrap();
    }, Duration::from_secs(2));

    window_test_op(&window, "Maximize", |w| {
        w.set_max_size(Extent2::new(99999999, 9999999)).unwrap();
        w.maximize().unwrap();
    }, Duration::from_secs(2));

    window_test_op(&window, "Unmaximize", |w| w.unmaximize().unwrap(), Duration::from_secs(2));

    window_test_op(&window, "Toggle maximize 1", |w| w.toggle_maximize().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Toggle maximize 2", |w| w.toggle_maximize().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Toggle maximize 3", |w| w.toggle_maximize().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Toggle maximize 4", |w| w.toggle_maximize().unwrap(), Duration::from_secs(2));

    window_test_op(&window, "Minimize", |w| w.minimize().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Unminimize", |w| w.unminimize().unwrap(), Duration::from_secs(2));

    window_test_op(&window, "Enter Fullscreen", |w| w.enter_fullscreen().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Fullscreen half opacity", |w| {
        w.set_opacity(0.5).unwrap();
    }, Duration::from_secs(2));
    window_test_op(&window, "Fullscreen zero opacity", |w| {
        w.set_opacity(0.).unwrap();
    }, Duration::from_secs(2));
    window_test_op(&window, "Leave Fullscreen", |w| {
        w.set_opacity(1.).unwrap();
        w.leave_fullscreen().unwrap()
    }, Duration::from_secs(2));

    window_test_op(&window, "Toggle Fullscreen 1", |w| w.toggle_fullscreen().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Toggle Fullscreen 2", |w| w.toggle_fullscreen().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Toggle Fullscreen 3", |w| w.toggle_fullscreen().unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Toggle Fullscreen 4", |w| w.toggle_fullscreen().unwrap(), Duration::from_secs(2));

    window_test_op(&window, "Demand Attention", |w| w.demand_attention().unwrap(), Duration::from_secs(2));

    window_test_op(&window, "Set Position", |w| w.set_position(Vec2::new(100, 100)).unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Set Size", |w| w.set_size(Extent2::new(600, 600)).unwrap(), Duration::from_secs(2));
    window_test_op(&window, "Set Position and Size", |w| {
        let r = Rect { x: 200, y: 200, w: 200, h: 200 };
        w.set_position_and_size(r).unwrap();
    }, Duration::from_secs(2));
}
