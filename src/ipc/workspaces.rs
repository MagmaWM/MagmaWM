use smithay::reexports::wayland_server::Dispatch;

use crate::utils::workspace::Workspaces as CompWorkspaces;

use super::{generated::workspaces::Workspaces, MagmaIpcManager, MagmaIpcHandler};

impl<D> Dispatch<Workspaces, (), D> for MagmaIpcManager
where
    D: Dispatch<Workspaces, ()>,
    D: MagmaIpcHandler,
    D: 'static, {
    fn request(
        _state: &mut D,
        _client: &smithay::reexports::wayland_server::Client,
        _resource: &Workspaces,
        _request: <Workspaces as smithay::reexports::wayland_server::Resource>::Request,
        _data: &(),
        _dhandle: &smithay::reexports::wayland_server::DisplayHandle,
        _data_init: &mut smithay::reexports::wayland_server::DataInit<'_, D>,
    ) {
        
    }
}

impl MagmaIpcManager {
    pub fn update_active_workspace(&mut self, id: u32) {
        for workspace_handle in self.workspace_handles.iter() {
            workspace_handle.active_workspace(id);
        }
    }

    pub fn update_occupied_workspaces(&mut self, workspaces: &mut CompWorkspaces) {
        for workspace_handle in self.workspace_handles.iter() {
            let mut occupied = vec![];
            for (id, workspace) in workspaces.iter().enumerate() {
                if workspace.windows().next().is_some() {
                    occupied.push(id as u8);
                }
            }
            workspace_handle.occupied_workspaces(occupied);
        }
    }
}
