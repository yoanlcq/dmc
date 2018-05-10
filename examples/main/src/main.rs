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
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use dmc::Event;
use app::{App, test::WhatNow};

fn main() {
    early::early();
    run_all_tests_and_report(app::App::default())
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

    println!();
    println!("--- FAILED TESTS ---");
    for (test, e) in failed.iter() {
        println!("- {:?}: {}", *test, e)
    }

    println!();
    println!("--- TEST RESULTS ---");
    println!("total   : {}", succeeded.len() + failed.len());
    println!("ok      : {}", succeeded.len());
    println!("failed  : {}", failed.len());
}
