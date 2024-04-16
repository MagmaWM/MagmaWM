use std::{fs::File, panic, thread};

use backtrace::Backtrace;
use chrono::Local;
use clap::{Parser, ValueEnum};
use tracing::{error, info};
use tracing_subscriber::{
    filter::{Directive, EnvFilter, LevelFilter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

use crate::backends::{udev, winit};

mod backends;
mod config;
#[cfg(feature = "debug")]
mod debug;
mod handlers;
mod protocols;
mod state;
mod utils;

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
    // TODO: maybe make these Paths to begin with?
    let log_dir = format!(
        "{}/.local/share/MagmaWM/",
        std::env::var("HOME").expect("this should always be set")
    );
    let log_file_name = format!("magma_{}.log", Local::now().format("%Y-%m-%d_%H:%M:%S"));
    let log_file_path = format!("{log_dir}/{log_file_name}");
    let log_link_path = format!("{log_dir}/latest.log");

    // create a new log file and symlink latest.log to it
    let log_file = File::create(&log_file_path).expect("Unable to create log file");
    // delete latest.log if it already exists
    if std::path::Path::new(&log_link_path).exists() {
        std::fs::remove_file(&log_link_path)
            .unwrap_or_else(|_| panic!("Unable to remove {log_link_path}"));
    }
    std::os::unix::fs::symlink(&log_file_path, log_link_path).expect("Unable to symlink log file");

    fn make_filter<S: AsRef<str>>(log_level: Option<S>, default: &Directive) -> EnvFilter {
        // filter using the --log flag if passed, otherwise use RUST_LOG, ignoring invalid strings
        match log_level {
            Some(log_level) => EnvFilter::builder()
                .with_default_directive(default.clone())
                .parse_lossy(log_level),
            None => EnvFilter::builder()
                .with_default_directive(default.clone())
                .from_env_lossy(),
        }
    }

    let args = Args::parse();

    let default = LevelFilter::INFO.into();

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(log_file)
        .with_filter(make_filter(args.log.as_ref(), &default));
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(make_filter(args.log.as_ref(), &default));

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .init();

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
