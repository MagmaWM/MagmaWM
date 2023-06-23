pub mod generated {
    use smithay::reexports::wayland_server;

    pub mod __interfaces {
        wayland_scanner::generate_interfaces!("magma-ipc/ipc.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_server_code!("magma-ipc/ipc.xml");
}
mod workspaces;
use smithay::reexports::wayland_server::{GlobalDispatch, Dispatch, DisplayHandle, Client, New, DataInit};

use self::generated::{magma_ipc::{MagmaIpc, Request}, workspaces::Workspaces};


pub struct MagmaIpcManager {
    pub workspace_handles: Vec<Workspaces>,
}

impl MagmaIpcManager {
    pub fn new<D>(display: &DisplayHandle) -> Self
    where
        D: GlobalDispatch<MagmaIpc, ()>,
        D: Dispatch<MagmaIpc, ()>,
        D: Dispatch<Workspaces, ()>,
        D: MagmaIpcHandler,
        D: 'static,
    {
        display.create_global::<D, MagmaIpc, _>(1, ());

        Self {
            workspace_handles: Vec::new(),
        }
    }
}

impl<D> GlobalDispatch<MagmaIpc, (), D> for MagmaIpcManager
where
    D: GlobalDispatch<MagmaIpc, ()>,
    D: Dispatch<MagmaIpc, ()>,
    D: Dispatch<Workspaces, ()>,
    D: MagmaIpcHandler,
    D: 'static,
{
    fn bind(
        _state: &mut D,
        _display: &DisplayHandle,
        _client: &Client,
        manager: New<MagmaIpc>,
        _manager_state: &(),
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(manager, ());
    }
}

impl<D> Dispatch<MagmaIpc, (), D> for MagmaIpcManager
where
    D: GlobalDispatch<MagmaIpc, ()>,
    D: Dispatch<MagmaIpc, ()>,
    D: Dispatch<Workspaces, ()>,
    D: MagmaIpcHandler,
    D: 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        _resource: &MagmaIpc,
        request: Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            Request::Workspaces { id } => state.register_workspace(data_init.init(id, ())),
        };
    }
}


#[macro_export]
macro_rules! delegate_magma_ipc {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ipc::generated::magma_ipc::MagmaIpc: ()
        ] => $crate::ipc::MagmaIpcManager);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ipc::generated::magma_ipc::MagmaIpc: ()
        ] => $crate::ipc::MagmaIpcManager);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::ipc::generated::workspaces::Workspaces: ()
        ] => $crate::ipc::MagmaIpcManager);
    };
}

pub trait MagmaIpcHandler {
    fn register_workspace(&mut self, workspace: Workspaces);
}