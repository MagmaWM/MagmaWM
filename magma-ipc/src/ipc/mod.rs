pub mod generated {
    use wayland_client;

    pub mod __interfaces {
        wayland_scanner::generate_interfaces!("./ipc.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("./ipc.xml");
}

pub mod workspaces;
