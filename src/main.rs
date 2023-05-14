use std::{panic, thread};

use backends::winit;
use backtrace::Backtrace;
use chrono::Local;
use tracing::error;

mod state;
mod backends;
mod handlers;
mod utils;
fn main() {
    let file_appender = tracing_appender::rolling::hourly("/tmp/magma/", format!("magma_{}.log", Local::now().to_string()));
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_writer(file_appender).with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().with_writer(file_appender).init();
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
