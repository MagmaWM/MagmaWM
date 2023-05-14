use std::{panic, thread};

use backends::winit;
use backtrace::Backtrace;
use chrono::Local;
use tracing::error;
use tracing_subscriber::fmt::writer::MakeWriterExt;

mod state;
mod backends;
mod handlers;
mod utils;
fn main() {
    let file_appender = tracing_appender::rolling::never(format!("{}/.local/share/MagmaEWM/", std::env::var("HOME").expect("this should always be set")), format!("magma_{}.log", Local::now().format("%Y-%m-%d_%H:%M:%S").to_string()));
    let log_appender = std::io::stdout.and(file_appender);
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_writer(log_appender).with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().with_writer(log_appender).init();
    }
    panic::set_hook(Box::new(move |info| {
        let backtrace = Backtrace::new();

        let thread = thread::current();
        let thread = thread.name().unwrap_or("<unnamed>");

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };

        match info.location() {
            Some(location) => {
                error!(
                    target: "panic", "thread '{}' panicked at '{}': {}:{}{:?}",
                    thread,
                    msg,
                    location.file(),
                    location.line(),
                    backtrace
                );
            }
            None => error!(
                target: "panic",
                "thread '{}' panicked at '{}'{:?}",
                thread,
                msg,
                backtrace
            ),
        }
    }));
    
    winit::init_winit();
}
