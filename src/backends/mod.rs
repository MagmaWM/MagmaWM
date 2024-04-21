use std::env;

pub mod udev;
pub mod winit;

pub fn init_backend_auto() {
    if env::var("WAYLAND_DISPLAY").is_ok() || env::var("DISPLAY").is_ok() {
        winit::init_winit();
    } else {
        udev::init_udev();
    }
}
