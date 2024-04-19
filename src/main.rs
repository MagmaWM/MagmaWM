use std::{panic, thread};

use backtrace::Backtrace;
use clap::{Parser, ValueEnum};
use tracing::{error, info};

use crate::{
    backends::{udev, winit},
    utils::log::init_logs,
};

mod backends;
mod config;
#[cfg(feature = "debug")]
mod debug;
mod handlers;
mod protocols;
mod state;
mod utils;
#[cfg(feature = "xwayland")]
mod xwayland;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "auto")]
    backend: Backend,
    /// Specify log level (fatal, error, warn, info, debug, trace)
    #[arg(long, name = "LEVEL")]
    log: Option<String>,
}

#[derive(Debug, Clone, ValueEnum)]
enum Backend {
    /// Run Magma as an X11 or Wayland client using winit
    Winit,
    /// Run Magma as a tty udev client (requires root if without logind)
    TtyUdev,
    /// Automatically select a backend
    Auto,
}

fn main() {
    let args = Args::parse();

    init_logs(args.log);

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
                    target: "panic",
                    "thread '{}' panicked at '{}': {}:{}{:?}",
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

    match args.backend {
        Backend::Winit => {
            info!("Starting magmawn with winit backend");
            winit::init_winit();
        }
        Backend::TtyUdev => {
            info!("Starting magma on a tty using udev");
            udev::init_udev();
        }
        Backend::Auto => backends::init_backend_auto(),
    }

    info!("Magma is shutting down");
}
