use std::env;
use env_logger;
use log::LevelFilter;

pub fn early() {
    setup_env();
    setup_log();
}

fn setup_env() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "trace");
    }
    env::set_var("RUST_BACKTRACE", "full");
}

fn setup_log() {
    use ::std::io::Write;

    let mut builder = env_logger::Builder::new();
    builder.format(|buf, record| {
        let s = format!("{}", record.level());
        let s = s.chars().next().unwrap();
        writeln!(buf, "[{}] {}", s, record.args())
    }).filter(None, LevelFilter::Debug);

    if let Ok(rust_log) = env::var("RUST_LOG") {
        builder.parse(&rust_log);
    }
    builder.init();
}

