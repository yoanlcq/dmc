#![deny(unused_must_use)]

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate dmc;

mod early;
mod app;
#[cfg(x11)]
mod app_x11;

use std::thread;
use std::env;
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use dmc::{Event, hint::{Hint, set_hint}};
use app::{App, test::WhatNow};

fn main() {
    early::early();
    #[cfg(x11)] set_hint(Hint::XlibXInitThreads).unwrap();
    #[cfg(x11)] set_hint(Hint::XlibDefaultErrorHandlers(false)).unwrap();
    let app = app::App::default();
    let args: Vec<_> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("simple") => run_simple_app(app),
        Some(_) => unimplemented!{},
        None => run_all_tests_and_report(app),
    }
}

#[cfg(feature="headless")]
fn run_simple_app(mut app: App) {}
#[cfg(not(feature="headless"))]
fn run_simple_app(mut app: App) {
    app.init_simple_app().unwrap();
    use ::dmc::device::{Key, Keysym};

    'main_loop: loop {
        for ev in app.pump_events() {
            match ev {
                Event::Quit | Event::WindowCloseRequested { .. } |
                Event::KeyboardKeyPressed { key: Key { sym: Some(Keysym::Esc), .. }, .. } 
                    => break 'main_loop,
                _ => (),
            }
        }
    }
}

fn wait_for_approval(app: &mut App) {
    let timeout = Duration::from_secs(4);
    let start = Instant::now();
    'wait_for_approval: while start.elapsed() < timeout {
        for ev in app.pump_events() {
            match ev {
                Event::KeyboardKeyPressed { .. } => break 'wait_for_approval,
                _ => (),
            }
        }
    }
}

fn run_all_tests_and_report(mut app: App) {
    let mut succeeded = HashSet::new();
    let mut failed = HashMap::new();

    for test in app::test::ALL_TESTS {
        info!("--- TEST {:?} --- ", *test);
        app.pump_events();
        let result = app.run_test(*test);
        app.pump_events();
        match result {
            Ok(what_now) => {
                match what_now {
                    WhatNow::RunNextTest => (),
                    WhatNow::WaitForApproval => wait_for_approval(&mut app),
                    WhatNow::Sleep(d) => thread::sleep(d),
                }
                info!("--- TEST {:?} OK ---", *test);
                succeeded.insert(*test);
            },
            Err(e) => {
                info!("--- TEST {:?} FAILED ({}) ---", *test, e);
                failed.insert(*test, e);
            },
        }
    }

    drop(app);

    if !failed.is_empty() {
        println!();
        println!("--- FAILED TESTS ---");
        for (test, e) in failed.iter() {
            println!("- {:?}: {}", *test, e)
        }
    }

    println!();
    println!("--- TEST RESULTS ---");
    println!("total   : {}", succeeded.len() + failed.len());
    println!("ok      : {}", succeeded.len());
    println!("failed  : {}", failed.len());
}
