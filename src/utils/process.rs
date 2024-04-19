use std::process::{Child, Command};
use tracing::{debug, info};

pub fn spawn(command: &str) -> Option<Child> {
    debug!("Spawning '{command}'");
    Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .map_err(|e| info!("Failed to spawn '{command}': {e}"))
        .ok()
}
