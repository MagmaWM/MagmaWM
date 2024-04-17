use chrono::Local;
use std::{
    fs,
    fs::File,
    os,
    path::{Path, PathBuf},
};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::INFO;

/// Initialize logging for the application.
///
/// Log files are in `~/.local/share/MagmaWM`
/// * Creates a timestamped log file: `magma_YYYY-MM-DD_HH:MM:SS.log`.
/// * Symlinks `latest.log` to the latest timestamped log file.
///
/// Logs are also printed to `stderr`.
///
/// Log levels are set by the following, in order of precedence:
/// * `log_level`
/// * The `RUST_LOG` environment variable
/// * `DEFAULT_LOG_LEVEL`
///
/// **Note:** Malformed values will result in no logs.
///
/// # Parameters
///
/// * `log_level`: The primary log level setting. Intended to be the value of the `--log=LOG_LEVEL`
/// flag.
pub fn init_logs<S: AsRef<str>>(log_level: Option<S>) {
    let home_dir = std::env::var("HOME").expect("$HOME is not set");
    let log_dir = PathBuf::from(home_dir).join(".local/share/MagmaWM/");

    let log_file_name = format!("magma_{}.log", Local::now().format("%Y-%m-%d_%H:%M:%S"));
    let log_file_path = log_dir.join(log_file_name);
    let log_link_path = log_dir.join("latest.log");

    // create a new log file and symlink latest.log to it
    let log_file = File::create(&log_file_path).expect("Unable to create log file");
    // delete latest.log if it already exists
    if Path::new(&log_link_path).exists() {
        fs::remove_file(&log_link_path)
            .unwrap_or_else(|_| panic!("Unable to remove {}", log_link_path.to_string_lossy()));
    }
    os::unix::fs::symlink(&log_file_path, log_link_path).expect("Unable to symlink log file");

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(log_file)
        .with_filter(filter(log_level.as_ref()));
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(filter(log_level));

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .init();
}

/// Creates a filter with the value of `log_level` if it is `Some`, or the `RUST_LOG` environment
/// variable if it is `None`, or `DEFAULT_LOG_LEVEL` if neither of the previous have a value.
///
/// **Note:** Malformed values will cause no logging at all.
///
/// This helper method exists to reduce repetion because `EnvFilter` does not implement `Clone`.
fn filter<S: AsRef<str>>(log_level: Option<S>) -> EnvFilter {
    // lossy means if the value is malformed, filter out everything
    match log_level {
        Some(log_level) => EnvFilter::builder().parse_lossy(log_level),
        None => EnvFilter::builder()
            .with_default_directive(DEFAULT_LOG_LEVEL.into())
            .from_env_lossy(),
    }
}
