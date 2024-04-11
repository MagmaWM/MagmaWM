use tracing::{error, info};

use std::{panic, thread};

use crate::backends::{udev, winit};
use backtrace::Backtrace;
use chrono::Local;
use tracing_subscriber::fmt::writer::MakeWriterExt;

mod backends;
mod config;
#[cfg(feature = "debug")]
mod debug;
mod handlers;
mod protocols;
mod state;
mod utils;

static POSSIBLE_BACKENDS: &[&str] = &[
    "--winit : Run magma as a X11 or Wayland client using winit.",
    "--tty-udev : Run magma as a tty udev client (requires root if without logind).",
];
fn main() {
    let log_dir = format!(
        "{}/.local/share/MagmaWM/",
        std::env::var("HOME").expect("this should always be set")
    );
    let log_file_name = format!("magma_{}.log", Local::now().format("%Y-%m-%d_%H:%M:%S"));
    let log_file_path = format!("{log_dir}/{log_file_name}");
    let log_link_path = format!("{log_dir}/latest.log");
    if std::path::Path::new(&log_link_path).exists() {
        std::fs::remove_file(&log_link_path).expect(&format!("Unable to remove {log_link_path}"));
    }
    std::os::unix::fs::symlink(log_file_path, log_link_path).expect("Unable to symlink log file");
    let file_appender = tracing_appender::rolling::never(&log_dir, log_file_name);
    let log_appender = std::io::stdout.and(file_appender);
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt()
            .with_writer(log_appender)
            .with_env_filter(env_filter)
            .init();
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

    let arg = ::std::env::args().nth(1);
    match arg.as_ref().map(|s| &s[..]) {
        Some("--winit") => {
            info!("Starting magmawn with winit backend");
            winit::init_winit();
        }
        Some("--tty-udev") => {
            info!("Starting magma on a tty using udev");
            udev::init_udev();
        }
        Some(other) => {
            error!("Unknown backend: {}", other);
            info!("Possible backends are:");
            for line in POSSIBLE_BACKENDS {
                println!("{}", line);
            }
        }
        None => {
            backends::init_backend_auto();
        }
    }

    info!("Magma is shutting down");
}
