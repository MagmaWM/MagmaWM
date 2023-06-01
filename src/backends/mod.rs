use std::env;

pub mod udev;
pub mod winit;

pub fn init_backend_from_name(name: &str) {
    match name {
        "udev" => {
            udev::init_udev();
        }
        "winit" => {
            winit::init_winit();
        }
        unknown => {
            tracing::error!("Attempted to start unknown backend: {}", unknown);
        }
    }
}

pub fn init_backend_auto() {
    if env::var("WAYLAND_DISPLAY").is_ok() || env::var("DISPLAY").is_ok() {
        init_backend_from_name("winit");
    } else {
        init_backend_from_name("udev");
    }
}
